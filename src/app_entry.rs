use pango::{Attribute, EllipsizeMode};
use std::collections::HashMap;
use gtk::{IconTheme, IconThemeExt, ListBoxRow, WidgetExt, Label, LabelExt, prelude::*, BoxBuilder, IconLookupFlags, ImageBuilder,
    Orientation};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use gio::{AppInfo, AppInfoExt};
use super::consts::*;
use glib::shell_unquote;
use futures::prelude::*;
use super::clone;

pub struct AppEntry {
    pub name: String,
    pub exe_range: Option<(u32, u32)>,
    pub info: AppInfo,
    pub label: Label,
    pub score: i64,
}

impl AppEntry {
    pub fn update_match(&mut self, pattern: &str, matcher: &SkimMatcherV2, exe_attrs: &Vec<Attribute>, highlight_attrs: &Vec<Attribute>) {
        self.score = if pattern.is_empty() {
            self.label.set_attributes(None);
            100
        } else if let Some((score, indices)) = matcher.fuzzy_indices(&self.name, pattern) {
            let attr_list = pango::AttrList::new();

            for i in indices {
                for attr in highlight_attrs {
                    let mut attr = attr.clone();
                    let i = i as u32;
                    attr.set_start_index(i);
                    attr.set_end_index(i+1);
                    attr_list.insert(attr);    
                }
            }

            self.label.set_attributes(Some(&attr_list));
            self.set_exe_markup(exe_attrs);
        
            score
        } else {
            0
        };
    }

    fn set_exe_markup(&self, exe_attrs: &Vec<Attribute>) {
        if let Some((lo, hi)) = self.exe_range {
            let attr_list = self.label.get_attributes().unwrap_or(pango::AttrList::new());
            for attr in exe_attrs {
                let mut attr = attr.clone();
                attr.set_start_index(lo);
                attr.set_end_index(hi);
                attr_list.insert(attr);    
            }
            self.label.set_attributes(Some(&attr_list));    
        }
    }
}

pub fn load_entries(exe_attrs: &Vec<Attribute>) -> HashMap<ListBoxRow, AppEntry> {
    let mut entries = HashMap::new();
    let icon_theme = IconTheme::get_default().unwrap();
    let icon_size = 64;
    let apps = gio::AppInfo::get_all();
    let main_context = glib::MainContext::default();

    for app in apps {
        let mut name = match app.get_display_name() {
            Some(n) if app.should_show() => n.to_string(),
            _=> continue
        };

        let mut exe_range = None;
        if let Some(filename) = app.get_executable().and_then(|p| p.file_name().map(|f| f.to_owned())) {
            if let Ok(filename) = shell_unquote(filename) {
                let filename = filename.to_string_lossy();
                if !name.to_lowercase().contains(&filename.to_lowercase()) {
                    exe_range = Some(((name.len()+1) as u32, (name.len()+1+filename.len()) as u32));
                    name = format!("{} {}", name, filename);
                }
            }
        }

        let label = gtk::LabelBuilder::new()
            .xalign(0.0f32)
            .label(&name)
            .wrap(true)
            .ellipsize(EllipsizeMode::End)
            .lines(2)
            .build();
        label.get_style_context().add_class(APP_LABEL_CLASS);

        let image = ImageBuilder::new().pixel_size(icon_size).build();
        if let Some(icon) = app
            .get_icon()
            .and_then(|icon| icon_theme.lookup_by_gicon(&icon, icon_size, IconLookupFlags::FORCE_SIZE))
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
            name,
            exe_range,
            info: app,
            label,
            score: 100,
        };
        app_entry.set_exe_markup(exe_attrs);
        entries.insert(row,app_entry);
    }
    entries
}
