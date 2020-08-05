use libc::LC_ALL;
use gdk::keys::constants;
use gio::{prelude::*};
use gtk::{prelude::*, ListBoxRow, WidgetExt};
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

fn activate(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);

    let config = Config::load();

    gtk_layer_shell::init_for_window(&window);
    gtk_layer_shell::set_keyboard_interactivity(&window, true);
    gtk_layer_shell::set_layer(&window, gtk_layer_shell::Layer::Overlay);

    if config.exclusive {
        gtk_layer_shell::auto_exclusive_zone_enable(&window);
    }

    // gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Left, 10);
    // gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Right, 10);
    // gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Top, 10);

    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Left, config.side == Side::Left);
    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Right, config.side == Side::Right);
    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Top, true);
    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Bottom, true);

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
        Inhibit(match event.get_keyval() {
            Escape => {
                window.close();
                true
            },
            Down | Tab if entry.has_focus() => {
                listbox.get_row_at_index(1).map(|row| listbox.select_row(Some(&row)));
                false
            },
            Up | Down | Page_Up | Page_Down | Tab | Shift_L | Shift_R | Control_L | Control_R
            | Alt_L | Alt_R | ISO_Left_Tab | Return => false,
            _ => {
                if !event.get_is_modifier() && !entry.has_focus() {
                    entry.grab_focus_without_selecting();
                }
                false
            }
        })
    }));

    let matcher = SkimMatcherV2::default();
    entry.connect_changed(clone!(entries, listbox => move |e| {
        let text = e.get_text();
        {
            let mut entries = entries.borrow_mut();
            for entry in entries.values_mut() {
                entry.update_match(&text, &matcher, &config);
            }
        }
        listbox.invalidate_filter();
        listbox.invalidate_sort();
        listbox.select_row(listbox.get_row_at_index(0).as_ref());
    }));

    entry.connect_activate(clone!(listbox => move |_| {
        if let Some(row) = listbox.get_row_at_index(0) {
            row.activate();
        }
    }));

    listbox.connect_row_activated(clone!(entries, window => move |_, r| {
        let e = entries.borrow();
        launch_app(&e[r].info);
        window.close();
    }));

    listbox.set_filter_func(Some(Box::new(clone!(entries => move |r| {
        let e = entries.borrow();
        e[r].score > 0
    }))));

    listbox.set_sort_func(Some(Box::new(clone!(entries => move |a, b| {
        let e = entries.borrow();
        let ea = &e[a];
        let eb = &e[b];
        (if ea.score == eb.score {
            string_collate(&e[a].name, &e[b].name)
        } else {
            eb.score.cmp(&ea.score)
        }) as i32
    }))));

    listbox.select_row(listbox.get_row_at_index(0).as_ref());

    window.add(&vbox);
    window.show_all()
}

fn main() {
    set_locale(LC_ALL, "");

    let application =
        gtk::Application::new(Some(APP_ID), Default::default())
            .expect("Initialization failed...");

    application.connect_activate(|app| {
        load_css();
        activate(app);
    });

    application.run(&args().collect::<Vec<_>>());
}
