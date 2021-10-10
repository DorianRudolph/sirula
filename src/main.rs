/*
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

use libc::LC_ALL;
use gdk::keys::constants;
use gio::{prelude::*};
use gtk::{prelude::*, ListBoxRow};
use std::env::args;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use fuzzy_matcher::skim::SkimMatcherV2;

mod consts;
use consts::*;

mod config;
use config::*;

mod util;
use util::*;

mod app_entry;
use app_entry::*;

mod locale;
use locale::*;

fn app_startup(application: &gtk::Application) {
    let config = Config::load();
    let cmd_prefix = config.command_prefix.clone();
    
    let window = gtk::ApplicationWindow::new(application);
    window.set_size_request(config.width, config.height);

    gtk_layer_shell::init_for_window(&window);
    gtk_layer_shell::set_keyboard_interactivity(&window, true);
    gtk_layer_shell::set_layer(&window, gtk_layer_shell::Layer::Overlay);

    if config.exclusive {
        gtk_layer_shell::auto_exclusive_zone_enable(&window);
    }

    gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Left, config.margin_left);
    gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Right, config.margin_right);
    gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Top, config.margin_top);
    gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Bottom, config.margin_bottom);

    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Left, config.anchor_left);
    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Right, config.anchor_right);
    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Top, config.anchor_top);
    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Bottom, config.anchor_bottom);

    window.set_decorated(false);
    window.set_app_paintable(true);

    let vbox = gtk::BoxBuilder::new()
        .name(ROOT_BOX_NAME)
        .orientation(gtk::Orientation::Vertical)
        .build();
    let entry = gtk::EntryBuilder::new().name(SEARCH_ENTRY_NAME).build(); // .width_request(300)
    vbox.pack_start(&entry, false, false, 0);

    let scroll = gtk::ScrolledWindowBuilder::new()
        .name(SCROLL_NAME)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    vbox.pack_end(&scroll, true, true, 0);

    let listbox = gtk::ListBoxBuilder::new().name(LISTBOX_NAME).build();
    scroll.add(&listbox);

    let entries = Rc::new(RefCell::new(load_entries(&config)));

    for (row, _) in &entries.borrow() as &HashMap<ListBoxRow, AppEntry> {
        listbox.add(row);
    }

    window.connect_key_press_event(clone!(entry, listbox => move |window, event| {
        use constants::*;
        #[allow(non_upper_case_globals)]
        Inhibit(match event.keyval() {
            Escape => {
                window.close();
                true
            },
            Down | Tab if entry.has_focus() => {
                listbox.row_at_index(1).map(|row| listbox.select_row(Some(&row)));
                false
            },
            Up | Down | Page_Up | Page_Down | Tab | Shift_L | Shift_R | Control_L | Control_R
            | Alt_L | Alt_R | ISO_Left_Tab | Return => false,
            _ => {
                if !event.is_modifier() && !entry.has_focus() {
                    entry.grab_focus_without_selecting();
                }
                false
            }
        })
    }));

    let matcher = SkimMatcherV2::default();
    entry.connect_changed(clone!(entries, listbox, cmd_prefix => move |e| {
        let text = e.text();
        let is_cmd = is_cmd(&text, &cmd_prefix);
        {
            let mut entries = entries.borrow_mut();
            for entry in entries.values_mut() {
                if is_cmd {
                    entry.hide(); // hide entries in command mode
                } else {
                    entry.update_match(&text, &matcher, &config);
                }
            }
        }
        listbox.invalidate_filter();
        listbox.invalidate_sort();
        listbox.select_row(listbox.row_at_index(0).as_ref());
    }));

    entry.connect_activate(clone!(listbox, window => move |e| {
        let text = e.text();
        if is_cmd(&text, &cmd_prefix) { // command execution direct
            let cmd_line = &text[cmd_prefix.len()..].trim();
            launch_cmd(cmd_line);
            window.close();
        } else if let Some(row) = listbox.row_at_index(0) {
            row.activate();
        }
    }));

    let min_score = 1;

    listbox.connect_row_activated(clone!(entries, window => move |_, r| {
        let es = entries.borrow();
        let e = &es[r];
        if e.score >= min_score {
            launch_app(&e.info);
            window.close();
        }
    }));

    listbox.set_filter_func(Some(Box::new(clone!(entries => move |r| {
        let e = entries.borrow();
        e[r].score >= min_score
    }))));

    listbox.set_sort_func(Some(Box::new(clone!(entries => move |a, b| {
        let e = entries.borrow();
        let ea = &e[a];
        let eb = &e[b];
        (if ea.score == eb.score {
            string_collate(&e[a].display_string, &e[b].display_string)
        } else {
            eb.score.cmp(&ea.score)
        }) as i32
    }))));

    listbox.select_row(listbox.row_at_index(0).as_ref());

    window.add(&vbox);
    window.show_all()
}

fn main() {
    set_locale(LC_ALL, "");

    let application = gtk::Application::new(Some(APP_ID), Default::default());

    application.connect_startup(|app| {
        load_css();
        app_startup(app);
    });

    application.connect_activate(|_| {
        //do nothing
    });

    application.run_with_args(&args().collect::<Vec<_>>());
}
