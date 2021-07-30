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

use pango::{Attribute, EllipsizeMode, AttrList};
use std::cmp::Ordering;
use std::collections::HashMap;
use gtk::{IconTheme, IconThemeExt, ListBoxRow, WidgetExt, Label, LabelExt, prelude::*, BoxBuilder, IconLookupFlags, ImageBuilder,
    Orientation};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use gio::{AppInfo, AppInfoExt};
use glib::shell_unquote;
use futures::prelude::*;
use crate::locale::string_collate;
use crate::util::load_history;

use super::{clone, consts::*, Config, Field};
use regex::RegexSet;

#[derive(Eq)]
pub struct AppEntry {
    pub display_string: String,
    pub search_string: String,
    pub extra_range: Option<(u32, u32)>,
    pub info: AppInfo,
    pub label: Label,
    pub score: i64,
    pub usage: usize,
}

impl AppEntry {
    pub fn update_match(&mut self, pattern: &str, matcher: &SkimMatcherV2, config: &Config) {
        self.set_markup(config);

        let attr_list = self.label.get_attributes().unwrap_or(AttrList::new());
        self.score = if pattern.is_empty() {
            self.label.set_attributes(None);
            100
        } else if let Some((score, indices)) = matcher.fuzzy_indices(&self.search_string, pattern) {
            for i in indices {
                if i < self.display_string.len() {
                    let i = i as u32;
                    add_attrs(&attr_list, &config.markup_highlight, i,  i + 1);
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

    fn set_markup(&self, config: &Config) {
        let attr_list = AttrList::new();

        add_attrs(&attr_list, &config.markup_default, 0, self.display_string.len() as u32);
        if let Some((lo, hi)) = self.extra_range {
            add_attrs(&attr_list, &config.markup_extra, lo, hi);   
        }
        self.label.set_attributes(Some(&attr_list));
    }
}

impl PartialEq for AppEntry {
    fn eq(&self, other: &Self) -> bool {
        self.score.eq(&other.score) && self.usage.eq(&other.usage)
    }
}

impl Ord for AppEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.score.cmp(&other.score) {
            Ordering::Equal => match self.usage.cmp(&other.usage) {
                Ordering::Equal => string_collate(&self.display_string, &other.display_string),
                ord => ord.reverse()
            }
            ord => ord.reverse()
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
        Field::Comment => app.get_description().map(Into::into),
        Field::Id => app.get_id().and_then(|s| s.to_string().strip_suffix(".desktop").map(Into::into)),
        Field::IdSuffix => app.get_id().and_then(|id| {
            let id = id.to_string();
            let parts : Vec<&str> = id.split('.').collect();
            parts.get(parts.len()-2).map(|s| s.to_string())
        }),
        Field::Executable => app.get_executable()
            .and_then(|p| p.file_name().map(ToOwned::to_owned))
            .and_then(|e| shell_unquote(e).ok())
            .map(|s| s.to_string_lossy().to_string()),
        //TODO: clean up command line from %
        Field::Commandline => app.get_commandline().map(|s| s.to_string_lossy().to_string()) 
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

pub fn load_entries(config: &Config) -> HashMap<ListBoxRow, AppEntry> {
    let mut entries = HashMap::new();
    let icon_theme = IconTheme::get_default().unwrap();
    let apps = gio::AppInfo::get_all();
    let main_context = glib::MainContext::default();
    let exclude = RegexSet::new(&config.exclude).expect("Invalid regex");

    let history = config.recent_first
        .then(load_history).flatten()
        .unwrap_or_else(HashMap::new);

    for app in apps {
        let name = match app.get_display_name() {
            Some(n) if app.should_show() => n.to_string(),
            _=> continue
        };

        if let Some(id) = app.get_id().map(|s| s.to_string()) {
            if exclude.is_match(&id) {
                continue
            }
        }

        let (display_string, extra_range) = if let Some(name) 
                = get_app_field(&app, Field::Id).and_then(|id| config.name_overrides.get(&id)) {
            let i = name.find('\r');
            (name.replace('\r', " "), i.map(|i| (i as u32 +1, name.len() as u32)))
        } else {
            let extra = config.extra_field.get(0).and_then(|f| get_app_field(&app, *f));
            match extra {
                Some(e) if (!config.hide_extra_if_contained || !name.to_lowercase().contains(&e.to_lowercase())) => (format!("{} {}", name, e), 
                    Some((name.len() as u32 + 1, name.len() as u32 + 1 + e.len() as u32))),
                _ => (name, None)
            }
        };

        let hidden = config.hidden_fields.iter()
            .map(|f| get_app_field(&app, *f).unwrap_or_default())
            .collect::<Vec<String>>().join(" ");
        
        let search_string = if hidden.is_empty() {
            display_string.clone()
        } else {
            format!("{} {}", display_string, hidden)
        };

        let label = gtk::LabelBuilder::new()
            .xalign(0.0f32)
            .label(&display_string)
            .wrap(true)
            .ellipsize(EllipsizeMode::End)
            .lines(config.lines)
            .build();
        label.get_style_context().add_class(APP_LABEL_CLASS);

        let image = ImageBuilder::new().pixel_size(config.icon_size).build();
        if let Some(icon) = app
            .get_icon()
            .and_then(|icon| icon_theme.lookup_by_gicon(&icon, config.icon_size, IconLookupFlags::FORCE_SIZE))
        {
            main_context.spawn_local(icon.load_icon_async_future().map(
                clone!(image => move |pb| {
                    if let Ok(pb) = pb {
                        image.set_from_pixbuf(Some(&pb));
                    }
                }),
            ));
        }
        image.get_style_context().add_class(APP_ICON_CLASS);

        let hbox = BoxBuilder::new()
            .orientation(Orientation::Horizontal)
            .build();
        hbox.pack_start(&image, false, false, 0);
        hbox.pack_end(&label, true, true, 0);

        let row = ListBoxRow::new();
        row.add(&hbox);
        row.get_style_context().add_class(APP_ROW_CLASS);

        let usage = history.get(&display_string).cloned().unwrap_or(0);

        let app_entry = AppEntry {
            display_string,
            search_string,
            extra_range,
            info: app,
            label,
            score: 100,
            usage,
        };
        app_entry.set_markup(config);
        entries.insert(row, app_entry);
    }
    entries
}
