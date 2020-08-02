use libc::{LC_COLLATE, LC_MESSAGES};
use std::str::FromStr;

mod locale;
use locale::*;
mod app_entry;
use app_entry::*;

use gio::prelude::*;
use gtk::prelude::*;
use gdk::keys::constants;

use std::env::args;

fn activate(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);

    window.connect_key_press_event(|w, e| {
        match e.get_keyval() {
            constants::Escape => {
                w.close();
                Inhibit(true)
            },
            _ => Inhibit(false),
        }
    });

    window.connect_show(|w| {
        println!("asdf");
    });

    // window.set_size_request(1000, 500);
    // window.resize(1000, 500);

    gtk_layer_shell::init_for_window(&window);
    gtk_layer_shell::set_keyboard_interactivity(&window, true);
    gtk_layer_shell::set_layer(&window, gtk_layer_shell::Layer::Overlay);
    // gtk_layer_shell::auto_exclusive_zone_enable(&window);

    // gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Left, 10);
    // gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Right, 10);
    // gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Top, 10);

    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Left, true);
    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Right, true);
    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Top, true);
    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Bottom, false);

    // Set up a widget
    // let label = gtk::Label::new(Some(""));
    // label.set_markup("<span font_desc=\"20.0\">GTK Layer Shell example!</span>");


    let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let entry = gtk::SearchEntry::new();
    entry.set_property_width_request(300);

    hbox.pack_start(&entry, false, false, 0);
    
    let scroll = gtk::ScrolledWindow::new(gtk::NONE_ADJUSTMENT, gtk::NONE_ADJUSTMENT);
    scroll.set_property_vscrollbar_policy(gtk::PolicyType::Never);
    let app_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);

    for i in 0..100 {
        let icon = gtk::Image::new();
        icon.set_from_icon_name(Some("ark"), gtk::IconSize::Dialog);
        app_box.pack_end(&icon, false, false, 0);
    }

    scroll.add(&app_box);

    hbox.pack_end(&scroll, true, true, 0);


    window.add(&hbox);
    window.set_border_width(12);
    window.show_all()
}

fn main() {
    let application =
        gtk::Application::new(Some("com.subgraph.gtk-layer-example"), Default::default())
            .expect("Initialization failed...");

    application.connect_activate(|app| {
        activate(app);
    });

    application.run(&args().collect::<Vec<_>>());
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
