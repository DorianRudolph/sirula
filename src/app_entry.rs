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
use std::collections::HashMap;
use gtk::{IconTheme, IconThemeExt, ListBoxRow, WidgetExt, Label, LabelExt, prelude::*, BoxBuilder, IconLookupFlags, ImageBuilder,
    Orientation};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use gio::{AppInfo, AppInfoExt};
use glib::shell_unquote;
use futures::prelude::*;
use super::{clone, consts::*, Config, Field};

pub struct AppEntry {
    pub display_string: String,
    pub search_string: String,
    pub extra_range: Option<(u32, u32)>,
    pub info: AppInfo,
    pub label: Label,
    pub score: i64,
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

    fn set_markup(&self, config: &Config) {
        let attr_list = AttrList::new();

        add_attrs(&attr_list, &config.markup_default, 0, self.display_string.len() as u32);
        if let Some((lo, hi)) = self.extra_range {
            add_attrs(&attr_list, &config.markup_extra, lo, hi);   
        }
        self.label.set_attributes(Some(&attr_list));
    }
}

// fn get_app_field(app: &AppInfo, field: Field) -> Option<String> {
//     match field {
//         Comment => app.get_description().map(Into::into),
//         Icon => app.get_icon().map(Into::into),
//         // GenericName => app.get_name(),
//         // Id,
//         // IdSuffix,
//         // Keywords,
//         // Exec,
//         // ExecName    
//     }
// }

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

    for app in apps {
        let name = match app.get_display_name() {
            Some(n) if app.should_show() => n.to_string(),
            _=> continue
        };


        let mut extra_range = None;
        let mut display_string = name.clone();
        if let Some(id) = app.get_id().map(|id| id.to_string()) {
            let id_parts : Vec<&str> = id.split('.').collect();
            if let Some(id) = id_parts.get(id_parts.len() - 2) {
                if !name.to_lowercase().contains(&id.to_lowercase()) {
                    extra_range = Some(((name.len()+1) as u32, (name.len()+1+id.len()) as u32));
                    display_string = format!("{} {}", name, id);
                }
            }
        }
        let search_string = display_string.clone();
        // if let Some(filename) = app.get_executable().and_then(|p| p.file_name().map(|f| f.to_owned())) {
        //     if let Ok(id) = shell_unquote(filename) {
        //         let id = id.to_string_lossy();
        //         println!("{:?}", app.get_id());
        //         if !name.to_lowercase().contains(&filename.to_lowercase()) {
        //             id_range = Some(((name.len()+1) as u32, (name.len()+1+filename.len()) as u32));
        //             name = format!("{} {}", name, filename);
        //         }
        //     }
        // }

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

        let app_entry = AppEntry {
            display_string,
            search_string,
            extra_range,
            info: app,
            label,
            score: 100,
        };
        app_entry.set_markup(config);
        entries.insert(row,app_entry);
    }
    entries
}
