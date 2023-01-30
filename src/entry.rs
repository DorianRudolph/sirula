use std::cmp::Ordering;
use std::collections::HashMap;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{fs, io};

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use gdk::pango::EllipsizeMode;
use gdk::prelude::*;
use gio::AppInfo;
use glib::{shell_unquote, SList};
use gtk::builders::{BoxBuilder, ImageBuilder, LabelBuilder};
use gtk::pango::{AttrList, Attribute};
use gtk::traits::{
    BoxExt, ContainerExt, IconThemeExt, ImageExt, LabelExt, StyleContextExt, WidgetExt,
};
use gtk::{IconLookupFlags, IconTheme, Label, ListBoxRow, Orientation};
use regex::RegexSet;
use titlecase::titlecase;

use crate::config::{Field, CONFIG};
use crate::consts::{APP_ICON_CLASS, APP_ROW_CLASS};
use crate::util::launch_app;
use crate::{history::HistoryData, locale::string_collate};

#[derive(Eq)]
pub struct Entry {
    pub id: String,
    pub display_string: String,
    pub search_string: String,
    pub label: Label,
    pub extra_range: Option<(u32, u32)>,
    pub score: i64,
    pub history: HistoryData,
    pub content: EntryContent,
}

impl Entry {
    pub fn load_applications(history: &HashMap<String, HistoryData>) -> HashMap<ListBoxRow, Self> {
        let mut entries = HashMap::new();
        let icon_theme = IconTheme::default().unwrap();
        let apps = gio::AppInfo::all();
        let exclude = RegexSet::new(&CONFIG.exclude).expect("Invalid regex");

        for app in apps {
            if !app.should_show() {
                continue;
            }
            let id = match app.id() {
                Some(id) => id.to_string(),
                None => continue,
            };
            if exclude.is_match(&id) {
                continue;
            }

            if let Some(entry) = Self::from_app_info(history, id, app) {
                let image = ImageBuilder::new().pixel_size(CONFIG.icon_size).build();
                if let EntryContent::Application(app) = &entry.content {
                    if let Some(icon) = app.icon() {
                        // Don't set the icon if it'd give us an ugly fallback icon
                        if icon_theme
                            .lookup_by_gicon(&icon, CONFIG.icon_size, IconLookupFlags::FORCE_SIZE)
                            .is_some()
                        {
                            image.set_from_gicon(&icon, gtk::IconSize::Menu);
                        }
                    }
                }
                image.style_context().add_class(APP_ICON_CLASS);

                let hbox = BoxBuilder::new()
                    .orientation(Orientation::Horizontal)
                    .build();
                hbox.pack_start(&image, false, false, 0);
                hbox.pack_end(&entry.label, true, true, 0);

                let row = ListBoxRow::new();
                row.add(&hbox);
                row.style_context().add_class(APP_ROW_CLASS);

                entries.insert(row, entry);
            }
        }
        entries
    }

    fn from_app_info(
        history: &HashMap<String, HistoryData>,
        id: String,
        app: AppInfo,
    ) -> Option<Self> {
        let name = app.display_name().to_string();

        let (display_string, extra_range) = if let Some(name) =
            get_app_field(&app, Field::Id).and_then(|id| CONFIG.name_overrides.get(&id))
        {
            let i = name.find('\r');
            (
                name.replace('\r', " "),
                i.map(|i| (i as u32 + 1, name.len() as u32)),
            )
        } else {
            let extra = CONFIG
                .extra_field
                .get(0)
                .and_then(|f| get_app_field(&app, *f));
            match extra {
                Some(e)
                    if (!CONFIG.hide_extra_if_contained
                        || !name.to_lowercase().contains(&e.to_lowercase())) =>
                {
                    (
                        format!("{} {}", name, e),
                        Some((
                            name.len() as u32 + 1,
                            name.len() as u32 + 1 + e.len() as u32,
                        )),
                    )
                }
                _ => (name, None),
            }
        };

        let hidden = CONFIG
            .hidden_fields
            .iter()
            .map(|f| get_app_field(&app, *f).unwrap_or_default())
            .collect::<Vec<String>>()
            .join(" ");

        let search_string = if hidden.is_empty() {
            display_string.clone()
        } else {
            format!("{} {}", display_string, hidden)
        };

        Some(Self::new(
            history,
            id,
            display_string,
            search_string,
            extra_range,
            100,
            EntryContent::Application(app),
        ))
    }

    pub fn load_scripts(
        history: &HashMap<String, HistoryData>,
        script_dir: PathBuf,
        init_score: i64,
    ) -> Result<HashMap<ListBoxRow, Self>, io::Error> {
        let mut entries = HashMap::new();
        for file in fs::read_dir(script_dir)? {
            let path = file?.path();
            if path.is_dir() {
                continue;
            }
            let entry = if let Some(entry) = Entry::from_path(history, path, init_score) {
                entry
            } else {
                continue;
            };

            let row = ListBoxRow::new();
            row.add(&entry.label);
            row.style_context().add_class(APP_ROW_CLASS);

            entries.insert(row, entry);
        }
        Ok(entries)
    }

    fn from_path<P: AsRef<Path>>(
        history: &HashMap<String, HistoryData>,
        path: P,
        init_score: i64,
    ) -> Option<Self> {
        let path = path.as_ref();
        let file_string = path.to_str()?.to_owned();

        // Format display string
        let display_string = titlecase(&path.file_stem()?.to_str()?.replace(&['_', '-'], " "));

        Some(Self::new(
            history,
            file_string.clone(),
            display_string.clone(),
            display_string + &file_string,
            None,
            init_score,
            EntryContent::Script(file_string),
        ))
    }

    pub fn from_stdin(history: &HashMap<String, HistoryData>) -> HashMap<ListBoxRow, Self> {
        let mut entries = HashMap::new();

        let mut buf = String::new();
        while let Ok(bytes) = io::stdin().read_line(&mut buf) {
            if bytes == 0 {
                break;
            }

            let entry = Entry::from_string(history, buf.trim().to_owned());
            buf.clear();

            let row = ListBoxRow::new();
            row.add(&entry.label);
            row.style_context().add_class(APP_ROW_CLASS);

            entries.insert(row, entry);
        }
        entries
    }

    fn from_string(history: &HashMap<String, HistoryData>, s: String) -> Self {
        Self::new(
            history,
            s.clone(),
            s.clone(),
            s.clone(),
            None,
            100,
            EntryContent::Stdin(s),
        )
    }

    fn new(
        history: &HashMap<String, HistoryData>,
        id: String,
        display_string: String,
        search_string: String,
        extra_range: Option<(u32, u32)>,
        init_score: i64,
        content: EntryContent,
    ) -> Self {
        let mut history_data = history.get(&id).copied().unwrap_or_default();

        // Remove unneeded history data
        if !CONFIG.recent_first {
            history_data.last_used = 0;
        }
        if !CONFIG.frequent_first {
            history_data.usage_count = 0;
        }

        let label = LabelBuilder::new()
            .xalign(0.0f32)
            .label(&display_string)
            .wrap(true)
            .ellipsize(EllipsizeMode::End)
            .lines(CONFIG.lines)
            .build();

        Entry {
            id,
            display_string,
            search_string,
            score: init_score,
            extra_range,
            history: history_data,
            label,
            content,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn is_application(&self) -> bool {
        self.content.is_application()
    }

    pub fn update_match(&mut self, pattern: &str, matcher: &SkimMatcherV2) {
        self.set_markup();

        let attr_list = self.label.attributes().unwrap_or_default();
        self.score = if pattern.is_empty() {
            self.label.set_attributes(None);
            100
        } else if let Some((score, indices)) = matcher.fuzzy_indices(&self.search_string, pattern) {
            for i in indices {
                if i < self.display_string.len() {
                    let i = i as u32;
                    add_attrs(&attr_list, &CONFIG.markup_highlight, i, i + 1);
                }
            }
            score
        } else {
            0
        };

        self.label.set_attributes(Some(&attr_list));
    }

    pub fn hide(&mut self) {
        self.score = 0;
    }

    pub fn hidden(&self) -> bool {
        0 == self.score
    }

    pub fn set_markup(&self) {
        let attr_list = AttrList::new();

        add_attrs(
            &attr_list,
            &CONFIG.markup_default,
            0,
            self.display_string.len() as u32,
        );
        if let Some((lo, hi)) = self.extra_range {
            add_attrs(&attr_list, &CONFIG.markup_extra, lo, hi);
        }
        self.label.set_attributes(Some(&attr_list));
    }

    pub fn act(&self) {
        self.content.act();
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.score.eq(&other.score)
    }
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.score.cmp(&other.score) {
            Ordering::Equal => match self.history.usage_count.cmp(&other.history.usage_count) {
                Ordering::Equal => match self.history.last_used.cmp(&other.history.last_used) {
                    Ordering::Equal => string_collate(&self.display_string, &other.display_string),
                    ord => ord.reverse(),
                },
                ord => ord.reverse(),
            },
            ord => ord.reverse(),
        }
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn add_attrs(list: &AttrList, attrs: &SList<Attribute>, start: u32, end: u32) {
    for attr in attrs.iter() {
        let mut attr = attr.clone();
        attr.set_start_index(start);
        attr.set_end_index(end);
        list.insert(attr);
    }
}

fn get_app_field(app: &AppInfo, field: Field) -> Option<String> {
    match field {
        Field::Comment => app.description().map(Into::into),
        Field::Id => app
            .id()
            .and_then(|s| s.to_string().strip_suffix(".desktop").map(Into::into)),
        Field::IdSuffix => app.id().and_then(|id| {
            let id = id.to_string();
            let parts: Vec<&str> = id.split('.').collect();
            parts.get(parts.len() - 2).map(|s| s.to_string())
        }),
        Field::Executable => app
            .executable()
            .file_name()
            .and_then(|e| shell_unquote(e).ok())
            .map(|s| s.to_string_lossy().to_string()),
        //TODO: clean up command line from %
        Field::Commandline => app.commandline().map(|s| s.to_string_lossy().to_string()),
    }
}

#[derive(PartialEq, Eq)]
pub enum EntryContent {
    Application(AppInfo),
    Script(String),
    Stdin(String),
}

impl EntryContent {
    fn act(&self) {
        match self {
            EntryContent::Application(app) => launch_app(app, CONFIG.term_command.as_deref()),
            EntryContent::Script(path) => {
                println!("Running {}", path);
                Command::new(path).exec();
            }
            EntryContent::Stdin(text) => println!("{}", text),
        }
    }

    fn is_application(&self) -> bool {
        match self {
            EntryContent::Application(_) => true,
            _ => false,
        }
    }
}
