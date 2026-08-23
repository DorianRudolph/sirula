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

use env_logger::Builder;
use fuzzy_matcher::skim::SkimMatcherV2;
use gdk::keys::constants;
use gio::prelude::*;
use gio::SimpleAction;
use gtk::{
    builders::{BoxBuilder, EntryBuilder, ListBoxBuilder, ScrolledWindowBuilder},
    prelude::*,
    CssProvider, ListBoxRow,
};
use libc::LC_ALL;
use log::info;

use std::{cell::RefCell, collections::HashMap, env::args, rc::Rc};

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

mod history;
use history::*;

fn app_startup(application: &gtk::Application, daemon_mode: bool) {
    let config = Rc::new(RefCell::new(Config::load()));
    let cmd_prefix = config.borrow().command_prefix.clone();

    let css_provider = {
        let provider = CssProvider::new();

        gtk::StyleContext::add_provider_for_screen(
            &gdk::Screen::default().expect("Error initializing gtk css provider."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        Rc::new(RefCell::new(provider))
    };
    load_css(css_provider.clone());

    let window = gtk::Window::builder()
        .application(application)
        .name("sirula")
        .build();
    window.set_size_request(config.borrow().width, config.borrow().height);

    gtk_layer_shell::init_for_window(&window);
    gtk_layer_shell::set_keyboard_interactivity(&window, true);
    gtk_layer_shell::set_layer(&window, gtk_layer_shell::Layer::Overlay);
    gtk_layer_shell::set_namespace(&window, "sirula");

    if config.borrow().exclusive {
        gtk_layer_shell::auto_exclusive_zone_enable(&window);
    }

    gtk_layer_shell::set_margin(
        &window,
        gtk_layer_shell::Edge::Left,
        config.borrow().margin_left,
    );
    gtk_layer_shell::set_margin(
        &window,
        gtk_layer_shell::Edge::Right,
        config.borrow().margin_right,
    );
    gtk_layer_shell::set_margin(
        &window,
        gtk_layer_shell::Edge::Top,
        config.borrow().margin_top,
    );
    gtk_layer_shell::set_margin(
        &window,
        gtk_layer_shell::Edge::Bottom,
        config.borrow().margin_bottom,
    );

    gtk_layer_shell::set_anchor(
        &window,
        gtk_layer_shell::Edge::Left,
        config.borrow().anchor_left,
    );
    gtk_layer_shell::set_anchor(
        &window,
        gtk_layer_shell::Edge::Right,
        config.borrow().anchor_right,
    );
    gtk_layer_shell::set_anchor(
        &window,
        gtk_layer_shell::Edge::Top,
        config.borrow().anchor_top,
    );
    gtk_layer_shell::set_anchor(
        &window,
        gtk_layer_shell::Edge::Bottom,
        config.borrow().anchor_bottom,
    );

    window.set_decorated(false);

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

    let history = Rc::new(RefCell::new(load_history(config.borrow().prune_history)));
    let entries = Rc::new(RefCell::new(load_entries(
        &config.borrow(),
        &history.borrow(),
    )));

    for row in (&entries.borrow() as &HashMap<ListBoxRow, AppEntry>).keys() {
        listbox.add(row);
    }

    let action_close = SimpleAction::new("reload", None);
    action_close.connect_activate(
        clone!(history, entries, entry, listbox, config, window, css_provider => move |_, _| {
            info!("reloading");
            {
                let mut config = config.borrow_mut();
                *config = Config::load();
            }

            let entries_new = load_entries(&config.borrow(), &history.borrow());

            for row in listbox.children() {
                listbox.remove(&row);
            }

            {
                entries.replace(entries_new);
            }

            for row in (&entries.borrow() as &HashMap<ListBoxRow, AppEntry>).keys() {
                listbox.add(row);
            }

            load_css(css_provider.clone());

            if window.is_visible() {
                listbox.select_row(listbox.row_at_index(0).as_ref());
                entry.emit_by_name::<()>("changed", &[]);
                window.show_all()
            }
        }),
    );

    let action_toggle = SimpleAction::new("toggle", None);
    action_toggle.connect_activate(clone!(window, daemon_mode, entry => move |_, _| {
        info!("toggling");
        if window.is_visible() {
            hide_or_close(daemon_mode, &window, &entry);
        } else {
            window.show_all()
        }
    }));

    application.add_action(&action_close);
    application.add_action(&action_toggle);

    fn hide_or_close(daemon_mode: bool, window: &gtk::Window, entry: &gtk::Entry) {
        if daemon_mode {
            window.hide();
            let cur_text = entry.text();
            if cur_text.is_empty() {
                entry.emit_by_name::<()>("changed", &[]);
            } else {
                entry.set_text("");
            }
            entry.grab_focus_without_selecting();
        } else {
            window.close();
        }
    }

    window.connect_key_press_event(
        clone!(entry, listbox, entries, daemon_mode, application => move |window, event| {
            use constants::*;
            #[allow(non_upper_case_globals)]
            Inhibit(match event.keyval() {
                r | R if event.state().contains(gdk::ModifierType::CONTROL_MASK) => {
                    application.activate_action("reload", None);
                    false
                },
                F5 => {
                    application.activate_action("reload", None);
                    false
                },
                Escape => {
                    hide_or_close(daemon_mode, window, &entry);
                    true
                },
                Down | KP_Down | Tab if entry.has_focus() => {
                    if let Some(r0) = listbox.row_at_index(0) {
                        let es = entries.borrow();
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
                Up | Down | KP_Up | KP_Down | Page_Up | Page_Down | KP_Page_Up | KP_Page_Down | Tab
                | Shift_L | Shift_R | Control_L | Control_R | Alt_L | Alt_R | ISO_Left_Tab | Return
                | KP_Enter => false,
                _ => {
                    if !event.is_modifier() && !entry.has_focus() {
                        entry.grab_focus_without_selecting();
                    }
                    false
                }
            })
        }),
    );

    if config.borrow().close_on_unfocus {
        window.connect_focus_out_event(clone!(entry, daemon_mode => move |window, _| {
            hide_or_close(daemon_mode, window, &entry);
            Inhibit(false)
        }));
    }

    let matcher = SkimMatcherV2::default();
    entry.connect_changed(clone!(entries, listbox, cmd_prefix, config => move |e| {
        let text = e.text();
        let is_cmd = is_cmd(&text, &cmd_prefix);
        {
            let mut entries = entries.borrow_mut();
            for entry in entries.values_mut() {
                if is_cmd {
                    entry.hide(); // hide entries in command mode
                } else {
                    entry.update_match(&text, &matcher, &config.borrow());
                }
            }
        }
        listbox.invalidate_filter();
        listbox.invalidate_sort();
        listbox.select_row(listbox.row_at_index(0).as_ref());
    }));

    entry.connect_activate(clone!(listbox, window, daemon_mode => move |e| {
        let text = e.text();
        if is_cmd(&text, &cmd_prefix) { // command execution direct
            let cmd_line = &text[cmd_prefix.len()..].trim();
            launch_cmd(cmd_line);
            hide_or_close(daemon_mode, &window, e);
        } else if let Some(row) = listbox.row_at_index(0) {
            row.activate();
        }
    }));

    listbox.connect_row_activated(
        clone!(config, entry, entries, window, history, daemon_mode => move |_, r| {
            {
                let es = entries.borrow();
                let e = &es[r];
                if !e.hidden() {
                    e.info.run(config.clone());

                    let mut history = history.borrow_mut();
                    update_history(&mut history, &format!("{}.desktop", e.info.id));
                    save_history(&history);
                }
            }
            hide_or_close(daemon_mode, &window, &entry);
        }),
    );

    listbox.set_filter_func(Some(Box::new(clone!(entries => move |r| {
        let e = entries.borrow();
        !e[r].hidden()
    }))));

    listbox.set_sort_func(Some(Box::new(clone!(entries => move |a, b| {
        let e = entries.borrow();
        e[a].cmp(&e[b]) as i32
    }))));

    listbox.select_row(listbox.row_at_index(0).as_ref());

    window.add(&vbox);
    application.connect_activate(clone!(window => move |_| {
        window.show_all()
    }));

    gtk::glib::source::unix_signal_add_local(
        libc::SIGUSR1,
        clone!(application => move || {
            application.activate_action("reload", None);
            glib::Continue(true)
        }),
    );

    if daemon_mode {
        if !config.borrow().disable_file_monitor {
            app_entry::desktop_entry::setup_monitor(application);
        }
        if config.borrow().autostart_apps {
            app_entry::desktop_entry::autostart_apps(config);
        }
    }
}

fn main() {
    Builder::from_env("SIRULA_LOG").init();
    info!("sirula {}", env!("CARGO_PKG_VERSION"));

    set_locale(LC_ALL, "");

    let arg_string_vec: Vec<String> = args().collect();
    let daemon = arg_string_vec.iter().any(|s| {
        s == "--daemon"
            || s == "--reload-or-restart"
            || (s.starts_with('-') && s.len() > 1 && &s[1..2] != "-" && s.contains('d')
                || s.contains('R'))
    });
    let app_flags = if daemon {
        gio::ApplicationFlags::HANDLES_COMMAND_LINE | gio::ApplicationFlags::IS_SERVICE
    } else {
        gio::ApplicationFlags::HANDLES_COMMAND_LINE
    };
    let application = gtk::Application::new(Some(APP_ID), app_flags);

    application.add_main_option(
        "daemon",
        glib::Char::from(b'd'),
        glib::OptionFlags::IN_MAIN,
        glib::OptionArg::None,
        "start up in daemon mode",
        None,
    );

    application.add_main_option(
        "reload",
        glib::Char::from(b'r'),
        glib::OptionFlags::IN_MAIN,
        glib::OptionArg::None,
        "reload sirula when in daemon mode",
        None,
    );

    application.add_main_option(
        "reload-or-restart",
        glib::Char::from(b'R'),
        glib::OptionFlags::IN_MAIN,
        glib::OptionArg::None,
        "reload sirula when in daemon mode or start the daemon if not",
        None,
    );

    application.add_main_option(
        "toggle",
        glib::Char::from(b't'),
        glib::OptionFlags::IN_MAIN,
        glib::OptionArg::None,
        "toggle sirula visibility when in daemon mode",
        None,
    );

    application.add_main_option(
        "quit",
        glib::Char::from(b'q'),
        glib::OptionFlags::IN_MAIN,
        glib::OptionArg::None,
        "quit sirula when in daemon mode",
        None,
    );

    // handled above
    application.connect_handle_local_options(|_, _| -1);

    application.connect_command_line(|app, cmd| {
        let args = cmd.options_dict();

        let reload = args.contains("reload");
        let reload_or_restart = args.contains("reload-or-restart");
        let toggle = args.contains("toggle");
        let quit = args.contains("quit");

        if cmd.is_remote() {
            if quit {
                app.quit();
                return 0;
            }
            if reload || reload_or_restart {
                app.activate_action("reload", None);
            }
            if toggle {
                app.activate_action("toggle", None);
            }
            if !(reload || toggle) {
                app.activate()
            }
        } else {
            if reload || toggle || quit {
                log::warn!("no daemon instance running, ignoring application flags");
            }
            app.activate()
        }

        0
    });

    application.connect_startup(clone!(daemon => move |app| app_startup(app, daemon)));

    application.run_with_args(&arg_string_vec);
}
