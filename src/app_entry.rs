/*
This file is part of sirula.

Copyright (C) 2020 Dorian Rudolph

sirula is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

sirula is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with sirula.  If not, see <https://www.gnu.org/licenses/>.
*/

use crate::locale::string_collate;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use gio::AppInfo;
use glib::shell_unquote;
use gtk::{
    builders::{BoxBuilder, ImageBuilder, LabelBuilder},
    prelude::*,
    IconLookupFlags, IconTheme, Label, ListBoxRow, Orientation,
};
use pango::{AttrList, Attribute, EllipsizeMode};
use std::cmp::Ordering;
use std::collections::HashMap;

use super::{consts::*, Config, Field, HistoryData};
use regex::RegexSet;

#[derive(Eq)]
pub struct AppEntry {
    pub display_string: String,
    pub search_string: String,
    pub extra_range: Option<(u32, u32)>,
    pub info: AppInfo,
    pub label: Label,
    pub score: i64,
    pub history: HistoryData,
}

impl AppEntry {
    pub fn update_match(&mut self, pattern: &str, matcher: &SkimMatcherV2, config: &Config) {
        self.set_markup(config);

        let attr_list = self.label.attributes().unwrap_or_default();
        self.score = if pattern.is_empty() {
            self.label.set_attributes(None);
            100
        } else if let Some((score, indices)) = matcher.fuzzy_indices(&self.search_string, pattern) {
            for i in indices {
                if i < self.display_string.len() {
                    let i = i as u32;
                    add_attrs(&attr_list, &config.markup_highlight, i, i + 1);
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

    fn set_markup(&self, config: &Config) {
        let attr_list = AttrList::new();

        add_attrs(
            &attr_list,
            &config.markup_default,
            0,
            self.display_string.len() as u32,
        );
        if let Some((lo, hi)) = self.extra_range {
            add_attrs(&attr_list, &config.markup_extra, lo, hi);
        }
        self.label.set_attributes(Some(&attr_list));
    }
}

impl PartialEq for AppEntry {
    fn eq(&self, other: &Self) -> bool {
        self.score.eq(&other.score) && self.history.eq(&other.history)
    }
}

impl Ord for AppEntry {
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

impl PartialOrd for AppEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
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
        //TODO: clean up command line from % for all what is not done in launch_app() in src/util.rx
        Field::Commandline => app.commandline().map(|s| s.to_string_lossy().to_string()),
    }
}

fn add_attrs(list: &AttrList, attrs: &Vec<Attribute>, start: u32, end: u32) {
    for attr in attrs {
        let mut attr = attr.clone();
        attr.set_start_index(start);
        attr.set_end_index(end);
        list.insert(attr);
    }
}

pub fn load_entries(
    config: &Config,
    history: &HashMap<String, HistoryData>,
) -> HashMap<ListBoxRow, AppEntry> {
    let mut entries = HashMap::new();
    let icon_theme = IconTheme::default().unwrap();
    let apps = gio::AppInfo::all();
    let exclude = RegexSet::new(&config.exclude).expect("Invalid regex");

    for app in apps {
        if !app.should_show() {
            continue;
        }

        let name = app.display_name().to_string();

        let id = match app.id() {
            Some(id) => id.to_string(),
            _ => continue,
        };

        if exclude.is_match(&id) {
            continue;
        }

        let (display_string, extra_range) = if let Some(name) =
            get_app_field(&app, Field::Id).and_then(|id| config.name_overrides.get(&id))
        {
            let i = name.find('\r');
            (
                name.replace('\r', " "),
                i.map(|i| (i as u32 + 1, name.len() as u32)),
            )
        } else {
            let extra = config
                .extra_field
                .get(0)
                .and_then(|f| get_app_field(&app, *f));
            match extra {
                Some(e)
                    if (!config.hide_extra_if_contained
                        || !name.to_lowercase().contains(&e.to_lowercase())) =>
                {
                    (
                        format!("{}{}{}",
                            name,
                            if config.extra_field_newline {"\n"} else {" "},
                            e
                        ),
                        Some((
                            name.len() as u32 + 1,
                            name.len() as u32 + 1 + e.len() as u32,
                        )),
                    )
                }
                _ => (name, None),
            }
        };

        let hidden = config
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

        let label = LabelBuilder::new()
            .xalign(0.0f32)
            .label(&display_string)
            .wrap(true)
            .ellipsize(EllipsizeMode::End)
            .lines(config.lines)
            .build();
        label.style_context().add_class(APP_LABEL_CLASS);

        let image = ImageBuilder::new().pixel_size(config.icon_size).build();
        if let Some(icon) = app.icon() {
            // Don't set the icon if it'd give us an ugly fallback icon
            if icon_theme
                .lookup_by_gicon(&icon, config.icon_size, IconLookupFlags::FORCE_SIZE)
                .is_some()
            {
                image.set_from_gicon(&icon, gtk::IconSize::Menu);
            }
        }
        image.style_context().add_class(APP_ICON_CLASS);

        let hbox = BoxBuilder::new()
            .orientation(Orientation::Horizontal)
            .build();
        hbox.pack_start(&image, false, false, 0);
        hbox.pack_end(&label, true, true, 0);

        let row = ListBoxRow::new();
        row.add(&hbox);
        row.style_context().add_class(APP_ROW_CLASS);

        let history_data = history.get(&id).copied().unwrap_or_default();
        let last_used = if config.recent_first {
            history_data.last_used
        } else {
            0
        };
        let usage_count = if config.frequent_first {
            history_data.usage_count
        } else {
            0
        };

        let app_entry = AppEntry {
            display_string,
            search_string,
            extra_range,
            info: app,
            label,
            score: 100,
            history: HistoryData {
                last_used,
                usage_count,
            },
        };
        app_entry.set_markup(config);
        entries.insert(row, app_entry);
    }
    entries
}
