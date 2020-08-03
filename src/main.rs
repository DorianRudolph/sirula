use libc::LC_ALL;

mod locale;
use locale::*;

use gdk::keys::constants;
use gio::prelude::*;
use gtk::prelude::*;
use gtk::{
    BoxBuilder, IconLookupFlags, IconTheme, IconThemeExt, Image, ImageBuilder,
    Label, LabelExt, ListBoxRow, Orientation,
};
use pango::EllipsizeMode;

use gio::{AppInfo};
use std::env::args;
use glib::variant::ToVariant;


use log::LevelFilter;
use std::io::Write;

use futures::prelude::*;

use log::debug;

use glib::prelude::*;
use glib::clone;
use glib::Variant;

struct AppEntry {
    name: String,
    info: AppInfo,
    label: Label,
    image: Image,
    row: ListBoxRow,
    score: i32,
}

fn load_entries() -> Vec<AppEntry> {
    debug!("begin load_entries");
    let mut entries = Vec::new();

    let icon_theme = IconTheme::get_default().unwrap();
    let icon_size = 64;

    let apps = gio::AppInfo::get_all();

    debug!("got all");

    let main_context = glib::MainContext::default();

    for app in apps {
        if !app.should_show() || app.get_display_name().is_none() {
            continue;
        }
        let name = app.get_display_name().unwrap().to_string();

        let label = gtk::LabelBuilder::new().xalign(0.0f32).label(&name).build();
        label.set_line_wrap(true);
        label.set_lines(2);
        label.set_ellipsize(EllipsizeMode::End);

        let image = ImageBuilder::new().pixel_size(icon_size).build();
        if let Some(icon) = app
            .get_icon()
            .map(|icon| icon_theme.lookup_by_gicon(&icon, icon_size, IconLookupFlags::FORCE_SIZE))
            .flatten()
        {
            main_context.spawn_local(icon.load_icon_async_future().map(clone!(@weak image => move |pb| {
                if let Ok(pb) = pb {
                    image.set_from_pixbuf(Some(&pb));
                }
            })));
        }

        let hbox = BoxBuilder::new()
            .orientation(Orientation::Horizontal)
            .build();
        hbox.pack_start(&image, false, false, 0);
        hbox.pack_end(&label, true, true, 0);

        let row = ListBoxRow::new();
        row.add(&hbox);

        entries.push(AppEntry {
            name,
            info: app,
            label,
            image,
            row,
            score: 100,
        });
    }

    entries.sort_by(|a, b| string_collate(&a.name, &b.name));

    debug!("built");

    entries
}

fn activate(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);

    window.connect_show(|w| {
        println!("asdf");
    });

    // window.set_size_request(1000, 500);
    // window.resize(1000, 500);

    gtk_layer_shell::init_for_window(&window);
    gtk_layer_shell::set_keyboard_interactivity(&window, true);
    gtk_layer_shell::set_layer(&window, gtk_layer_shell::Layer::Overlay);
    gtk_layer_shell::auto_exclusive_zone_enable(&window);

    // gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Left, 10);
    // gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Right, 10);
    // gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Top, 10);

    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Left, false);
    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Right, true);
    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Top, true);
    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Bottom, true);

    // Set up a widget
    // let label = gtk::Label::new(Some(""));
    // label.set_markup("<span font_desc=\"20.0\">GTK Layer Shell example!</span>");

    let vbox = gtk::BoxBuilder::new()
        .name("rootbox")
        .orientation(gtk::Orientation::Vertical)
        .build();
    let entry = gtk::EntryBuilder::new()
        .name("search")
        //.width_request(300)
        .build();
    vbox.pack_start(&entry, false, false, 0);

    let scroll = gtk::ScrolledWindowBuilder::new()
        .name("scroll")
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    vbox.pack_end(&scroll, true, true, 0);

    let listbox = gtk::ListBoxBuilder::new().name("listbox").build();
    scroll.add(&listbox);

    let entries = load_entries();

    let mut i = 0;
    for entry in entries {
        // entry.row.set_action_target_value(Some(&i.to_variant()));
        listbox.add(&entry.row);
        i += 1;
    }

    // let move_entry = clone!(@weak listbox, @weak window => @default-panic, move |dir| {
    //     listbox.grab_focus();
    //     window.child_focus(dir);
    //     true
    // });

    let entry_clone = entry.clone();
    window.connect_key_press_event(  move |window, event| {
        // println!("{:?} {:?}", event.get_keycode(), event.get_keyval().name());
        use constants::*;
        #[allow(non_upper_case_globals)]
        Inhibit(match event.get_keyval() {
            Escape => {
                window.close();
                true
            }
            Up | Down | Page_Up | Page_Down | Tab | Shift_L | Shift_R | Control_L | Control_R
            | Alt_L | Alt_R | Return  => false,
            _ => {
                if !event.get_is_modifier() && !entry_clone.has_focus() {
                    entry_clone.grab_focus_without_selecting();
                }
                false
            }
        })
    });

    // entry.connect_changed(|e| {});
    listbox.connect_row_activated(|_, r| {
        println!("activate");
        //println!("{:?}", r.get_action_target_value().unwrap().get::<i32>());
    });

    // listbox.set_filter_func(Some(Box::new(|r| {
    //     r.get_index() > 10
    // })));

    // listbox.set_sort_func(Some(Box::new(|a, b| {
    //     a.get_index() - b.get_index()
    // })));

    window.add(&vbox);
    window.show_all()
}

fn main() {
    let mut builder = env_logger::Builder::from_default_env();

    builder
        .format(|buf, record| {
            writeln!(
                buf,
                "{} | {} | {}",
                buf.timestamp_millis(),
                record.level(),
                record.args()
            )
        })
        .filter(None, LevelFilter::Debug)
        .init();

    env_logger::Builder::new()
        .format(|buf, record| {
            let ts = buf.timestamp();
            writeln!(buf, "{}: {}: {}", ts, record.level(), record.args())
        })
        .build();

    set_locale(LC_ALL, "");

    let application =
        gtk::Application::new(Some("com.subgraph.gtk-layer-example"), Default::default())
            .expect("Initialization failed...");

    application.connect_activate(|app| {
        activate(app);
    });

    application.run(&args().collect::<Vec<_>>());
}
