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

use clap::Parser;
use fuzzy_matcher::skim::SkimMatcherV2;
use gdk::keys::constants;
use gio::prelude::*;
use gtk::{
    builders::{BoxBuilder, EntryBuilder, ListBoxBuilder, ScrolledWindowBuilder},
    prelude::*,
    ApplicationWindow, ListBox, ListBoxRow,
};
use libc::LC_ALL;
use std::{cell::RefCell, collections::HashMap, path::PathBuf, rc::Rc};

mod consts;
use consts::*;

mod config;
use config::*;

mod util;
use util::*;

mod entry;
use entry::*;

mod locale;
use locale::*;

mod history;
use history::*;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Option<SubCommand>,

    #[arg(raw = true)]
    gtk_args: Vec<String>,
}

#[derive(clap::Subcommand, Debug, Default, Clone)]
enum SubCommand {
    /// Launch an application (default action; uses .desktop files)
    #[default]
    Apps,

    /// Launch a script from a user-defined list of scripts
    Scripts {
        /// A directory of user-defined scripts
        #[arg(short, long)]
        script_dir: Option<String>,
    },

    /// Like dmenu; takes a list of inputs from stdin and prints the selected option to stdout
    Dmenu,
}

fn main() {
    set_locale(LC_ALL, "");

    // Parse command line arguments
    let args = Args::parse();

    let application = gtk::Application::new(Some(APP_ID), Default::default());

    {
        let command = args.command.clone().unwrap_or_default();
        application.connect_startup(move |app| {
            load_css();
            app_startup(app, &command);
        });
    }

    application.connect_activate(|_| {
        //do nothing
    });

    // Run the application
    let mut gtk_app_args = vec![std::env::args().into_iter().next().unwrap()];
    gtk_app_args.extend(args.gtk_args);
    application.run_with_args(&gtk_app_args);
}

fn app_startup(application: &gtk::Application, command: &SubCommand) {
    let (window, listbox, entry) = window_init(application);
    // App data
    let history = Rc::new(RefCell::new(load_history()));
    let entries = Rc::new(RefCell::new(match command {
        SubCommand::Apps => {
            let mut entries = Entry::load_applications(&history.borrow());
            if let Some(script_dir) = get_config_file(DEFAULT_SCRIPTS_DIR) {
                entries.extend(
                    Entry::load_scripts(&history.borrow(), script_dir, 0)
                        .expect("File system error"),
                );
            }
            entries
        }
        SubCommand::Scripts { script_dir } => {
            let script_dir = script_dir
                .clone()
                .map(|s| PathBuf::from(s))
                .or_else(|| get_config_file(DEFAULT_SCRIPTS_DIR))
                .expect("No script directory found");

            Entry::load_scripts(&history.borrow(), script_dir, 0).expect("File system error")
        }
        SubCommand::Dmenu => Entry::from_stdin(&history.borrow()),
    }));

    // Used for switching to scripts from an application launcher context
    let handle_script_prefix = match command {
        SubCommand::Apps => true,
        _ => false,
    };

    // Populate current entries
    for row in (&entries.borrow() as &HashMap<ListBoxRow, Entry>).keys() {
        listbox.add(row);
    }

    window.connect_key_press_event(clone!(entry, listbox, entries => move |window, event| {
        use constants::*;
        #[allow(non_upper_case_globals)]
        Inhibit(match event.keyval() {
            Escape => {
                window.close();
                true
            },
            Down | Tab if entry.has_focus() => {
                if let Some(r0) = listbox.row_at_index(0) {
                    let es = entries.borrow_mut();
                    if r0.is_selected() {
                        if let Some(r1) = listbox.row_at_index(1) {
                            if let Some(app_entry) = es.get(&r1) {
                                if !app_entry.hidden() {
                                    listbox.select_row(Some(&r1));
                                }
                            }
                        }
                    } else if let Some(app_entry) = es.get(&r0) {
                        if !app_entry.hidden() {
                            listbox.select_row(Some(&r0));
                        }
                    }
                }
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
    entry.connect_changed(clone!(entries, listbox => move |e| {
        let text = e.text();
        {
            let mut entries = entries.borrow_mut();
            // Hide everything when typing a command
            if has_prefix(&text, &CONFIG.command_prefix) {
                for entry in entries.values_mut() {
                    entry.hide();
                }

            // Possibly load scripts when in application launcher mode
            } else if handle_script_prefix {
                if has_prefix(&text, &CONFIG.script_prefix) {
                    // Filter scripts, hide applications
                    let query_text = text[CONFIG.script_prefix.len()..].trim();
                    for entry in entries.values_mut() {
                        if entry.is_application() {
                            entry.hide();
                        } else {
                            entry.update_match(&query_text, &matcher);
                        }
                    }
                } else {
                    // Filter applications, hide scripts
                    for entry in entries.values_mut() {
                        if entry.is_application() {
                            entry.update_match(&text, &matcher);
                        } else {
                            entry.hide();
                        }
                    }
                }
            } else {
                // Filter everything
                for entry in entries.values_mut() {
                    entry.update_match(&text, &matcher);
                }
            }
        }
        listbox.invalidate_filter();
        listbox.invalidate_sort();
        listbox.select_row(listbox.row_at_index(0).as_ref());
    }));

    entry.connect_activate(clone!(listbox, window => move |e| {
        let text = e.text();
        if has_prefix(&text, &CONFIG.command_prefix) { // command execution direct
            let cmd_line = &text[CONFIG.command_prefix.len()..].trim();
            launch_cmd(cmd_line);
            window.close();
        } else if let Some(row) = listbox.row_at_index(0) {
            row.activate();
        }
    }));

    listbox.connect_row_activated(clone!(entries, window, history => move |_, r| {
        let es = entries.borrow();
        let e = &es[r];
        if !e.hidden() {
            e.act();

            let mut history = history.borrow_mut();
            update_history(&mut history, &e.id());
            save_history(&history);

            window.close();
        }
    }));

    listbox.set_filter_func(Some(Box::new(clone!(entries => move |r| {
        let e = entries.borrow();
        !e[r].hidden()
    }))));

    listbox.set_sort_func(Some(Box::new(clone!(entries => move |a, b| {
        let e = entries.borrow();
        e[a].cmp(&e[b]) as i32
    }))));

    listbox.select_row(listbox.row_at_index(0).as_ref());

    window.show_all()
}

fn window_init(application: &gtk::Application) -> (ApplicationWindow, ListBox, gtk::Entry) {
    let window = gtk::ApplicationWindow::new(application);
    window.set_size_request(CONFIG.width, CONFIG.height);

    gtk_layer_shell::init_for_window(&window);
    gtk_layer_shell::set_keyboard_interactivity(&window, true);
    gtk_layer_shell::set_layer(&window, gtk_layer_shell::Layer::Overlay);

    if CONFIG.exclusive {
        gtk_layer_shell::auto_exclusive_zone_enable(&window);
    }

    gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Left, CONFIG.margin_left);
    gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Right, CONFIG.margin_right);
    gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Top, CONFIG.margin_top);
    gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Bottom, CONFIG.margin_bottom);

    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Left, CONFIG.anchor_left);
    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Right, CONFIG.anchor_right);
    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Top, CONFIG.anchor_top);
    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Bottom, CONFIG.anchor_bottom);

    window.set_decorated(false);
    window.set_app_paintable(true);

    let vbox = BoxBuilder::new()
        .name(ROOT_BOX_NAME)
        .orientation(gtk::Orientation::Vertical)
        .build();
    let entry = EntryBuilder::new().name(SEARCH_ENTRY_NAME).build(); // .width_request(300)
    vbox.pack_start(&entry, false, false, 0);

    let scroll = ScrolledWindowBuilder::new()
        .name(SCROLL_NAME)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    vbox.pack_end(&scroll, true, true, 0);

    let listbox = ListBoxBuilder::new().name(LISTBOX_NAME).build();
    scroll.add(&listbox);
    window.add(&vbox);

    (window, listbox, entry)
}
