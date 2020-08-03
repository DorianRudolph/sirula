use libc::{LC_COLLATE, LC_MESSAGES, LC_ALL};
use std::str::FromStr;
use std::path::Path;

mod locale;
use locale::*;
mod app_entry;
use app_entry::*;

use gio::prelude::*;
use gtk::prelude::*;
use gtk::{LabelExt, IconLookupFlags, ImageBuilder, Label, Image, Widget, BoxBuilder, Orientation, IconTheme, IconThemeExt};
use gdk::keys::constants;
use pango::EllipsizeMode;

use std::env::args;
use std::borrow::Borrow;
use gio::{Icon, AppInfo};

use gdk_pixbuf::Pixbuf;

struct AppEntry {
    name: String,
    info: AppInfo,
    label: Label,
    image: Image,
    hbox: gtk::Box,
    score: i32,
}

fn load_entries() -> Vec<AppEntry> {
    let mut entries = Vec::new();

    let icon_theme = IconTheme::get_default().unwrap();
    let icon_size = 32;

    let apps = gio::AppInfo::get_all();
    for app in apps {
        if !app.should_show() || app.get_display_name().is_none() {
            continue
        }
        let name = app.get_display_name().unwrap().to_string();

        let label = gtk::LabelBuilder::new().xalign(0.0f32).label(&name).build();
        // label.set_line_wrap(true);
        // label.set_lines(2);
        label.set_ellipsize(EllipsizeMode::End);

        let icon = app.get_icon()
            .map(|icon| icon_theme.lookup_by_gicon(&icon, icon_size, IconLookupFlags::FORCE_SIZE)).flatten()
            .map(|icon| icon.load_icon().ok()).flatten();

        let image = match icon {
            Some(icon) => ImageBuilder::new().pixbuf(&icon),
            _ => ImageBuilder::new().pixel_size(icon_size),
        }.build();

        let hbox = BoxBuilder::new().orientation(Orientation::Horizontal).build();
        hbox.pack_start(&image, false, false, 0);
        hbox.pack_end(&label, true, true, 0);

        entries.push(AppEntry {
            name,
            info: app,
            label,
            image,
            hbox: hbox,
            score: 100
        });
    }

    entries.sort_by(|a, b| string_collate(&a.name, &b.name));
    // apps.sort_by(|a, b| string_collate(a.get_display_name().unwrap()))

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

    let vbox = gtk::BoxBuilder::new().name("rootbox").orientation(gtk::Orientation::Vertical).build();
    let entry = gtk::EntryBuilder::new().name("search").width_request(300).build();
    vbox.pack_start(&entry, false, false, 0);

    let scroll = gtk::ScrolledWindowBuilder::new().name("scroll").hscrollbar_policy(gtk::PolicyType::Never).build();
    vbox.pack_end(&scroll, true, true, 0);

    let listbox = gtk::ListBoxBuilder::new().name("listbox").build();
    scroll.add(&listbox);

    let entries = load_entries();
    for entry in entries {
        listbox.add(&entry.hbox);
    }

    // let mut apps = gio::AppInfo::get_all();
    // apps.retain(|app| app.get_display_name().is_some());
    // // apps.sort_by(|a, b| string_collate(a.get_display_name().unwrap()))

    // for app in apps {
    //     if !app.should_show() {
    //         continue;
    //     }
    //     println!("{:?} {:?}", app.get_display_name(), app.get_description());
    //
    //     let name = app.get_display_name().unwrap().to_string();
    //     let label = gtk::LabelBuilder::new().xalign(0.0f32).label(&name).build();
    //     label.set_line_wrap(true);
    //     label.set_lines(2);
    //     label.set_ellipsize(EllipsizeMode::End);
    //
    //     // let icon_size = 32;
    //     // let pixbuf = match e.icon.to_owned() {
    //     //     Some(ico) if Path::new(&ico).is_absolute() => Pixbuf::from_file_at_scale(ico, icon_size, icon_size, true).ok(),
    //     //     Some(ico) => match icon_theme.load_icon("asdf", icon_size, IconLookupFlags::FORCE_SIZE) { Ok(pixbuf) => pixbuf, _ => None }
    //     //     _ => None,
    //     // };
    //     // let icon = match pixbuf {
    //     //     Some(pixbuf) => ImageBuilder::new().pixbuf(&pixbuf).build(),
    //     //     _ => ImageBuilder::new().pixel_size(icon_size).icon_name(&e.icon.unwrap_or("".to_string())).build()
    //     // };
    //
    //     // let a = match e.icon {
    //     //     Some(path) if Path::new(&path).is_absolute() => {
    //     //
    //     //     },
    //     //     _ => 2
    //     // };
    //     // let icon = gtk::ImageBuilder::new().pixel_size(32).build();
    //     // // icon.set_from_icon_name(e.icon.as_ref().map(String::as_str), gtk::IconSize::Dnd);
    //     // if let Some(ico) = e.icon {
    //     //     icon.set_from_file(ico);
    //     //     icon.set_pixel_size(32);
    //     // }
    //
    //     let hbox = gtk::BoxBuilder::new().orientation(gtk::Orientation::Horizontal).build();
    //     // hbox.pack_start(&icon, false, false, 0);
    //     hbox.pack_end(&label, true, true, 0);
    //
    //     listbox.add(&hbox);
    // }

    let entry2 = entry.clone();
    window.connect_key_press_event(move |w, e| {
        if !entry2.has_focus() {
            entry2.grab_focus_without_selecting();
        }
        match e.get_keyval() {
            constants::Escape => {
                w.close();
                Inhibit(true)
            },
            _ => Inhibit(false),
        }
    });

    // let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    // let entry = gtk::SearchEntry::new();
    // entry.set_property_width_request(300);

    // hbox.pack_start(&entry, false, false, 0);
    
    // let scroll = gtk::ScrolledWindow::new(gtk::NONE_ADJUSTMENT, gtk::NONE_ADJUSTMENT);
    // scroll.set_property_vscrollbar_policy(gtk::PolicyType::Never);
    // let app_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);

    // for i in 0..100 {
    //     let icon = gtk::Image::new();
    //     icon.set_from_icon_name(Some(if i % 2 == 0 {"ark"} else {"firefox"}), gtk::IconSize::Dialog);
    //     app_box.pack_end(&icon, false, false, 0);
    // }

    // scroll.add(&app_box);

    // hbox.pack_end(&scroll, true, true, 0);


    window.add(&vbox);
    window.show_all()
}

fn main() {
    set_locale(LC_ALL, "");

    let application =
        gtk::Application::new(Some("com.subgraph.gtk-layer-example"), Default::default())
            .expect("Initialization failed...");

    application.connect_activate(|app| {
        activate(app);
    });

    application.run(&args().collect::<Vec<_>>());
}

fn get_entries() -> Vec<ApplicationEntry> {
    let locale = Locale::from_str(&get_locale(LC_MESSAGES).unwrap()).unwrap();
    let locale_strings = get_locale_strings(&locale);

    let mut entries = ApplicationEntry::parse_all(&locale_strings);
    entries.sort_by(|a, b| string_collate(&a.name, &b.name));
    entries
}

#[allow(dead_code)]
fn old_main() -> Result<(), &'static str> {
    set_locale(LC_MESSAGES, "");
    set_locale(LC_COLLATE, "");

    let locale = Locale::from_str(&get_locale(LC_MESSAGES).unwrap()).unwrap();
    let locale_strings = get_locale_strings(&locale);

    let mut entries = ApplicationEntry::parse_all(&locale_strings);
    entries.sort_by(|a, b| string_collate(&a.name, &b.name));

    for e in entries {
        println!("{:?}", e);
    }

    Ok(())
}
