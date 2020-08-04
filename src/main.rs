use libc::LC_ALL;

mod locale;
use locale::*;

use gdk::keys::constants;
use gio::prelude::*;
use gtk::prelude::*;
use gtk::{
    BoxBuilder, IconLookupFlags, IconTheme, IconThemeExt, ImageBuilder, Label, LabelExt,
    ListBoxRow, Orientation,
};
use pango::EllipsizeMode;

use gio::AppInfo;
use std::env::args;

use log::LevelFilter;
use std::{cell::RefCell, collections::HashMap, io::Write, rc::Rc};

use futures::prelude::*;

use log::debug;

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

struct AppEntry {
    name: String,
    info: AppInfo,
    label: Label,
    score: i64,
}

macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

fn load_entries() -> HashMap<ListBoxRow, AppEntry> {
    debug!("begin load_entries");
    let mut entries = HashMap::new();

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
            main_context.spawn_local(icon.load_icon_async_future().map(
                clone!(image => move |pb| {
                    if let Ok(pb) = pb {
                        image.set_from_pixbuf(Some(&pb));
                    }
                }),
            ));
        }

        let hbox = BoxBuilder::new()
            .orientation(Orientation::Horizontal)
            .build();
        hbox.pack_start(&image, false, false, 0);
        hbox.pack_end(&label, true, true, 0);

        let row = ListBoxRow::new();
        row.add(&hbox);

        entries.insert(
            row,
            AppEntry {
                name,
                info: app,
                label,
                score: 100,
            },
        );
    }

    debug!("built");

    entries
}

fn activate(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);

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

    let vbox = gtk::BoxBuilder::new()
        .name("rootbox")
        .orientation(gtk::Orientation::Vertical)
        .build();
    let entry = gtk::EntryBuilder::new().name("search").build();
    vbox.pack_start(&entry, false, false, 0);

    let scroll = gtk::ScrolledWindowBuilder::new()
        .name("scroll")
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    vbox.pack_end(&scroll, true, true, 0);

    let listbox = gtk::ListBoxBuilder::new().name("listbox").build();
    scroll.add(&listbox);

    let entries = Rc::new(RefCell::new(load_entries()));

    for (row, _) in &entries.borrow() as &HashMap<ListBoxRow, AppEntry> {
        listbox.add(row);
        // listbox.add(&LabelBuilder::new().build());
    }

    // let move_entry = clone!(@weak listbox, @weak window => @default-panic, move |dir| {
    //     listbox.grab_focus();
    //     window.child_focus(dir);
    //     true
    // });

    let entry_clone = entry.clone();
    window.connect_key_press_event(move |window, event| {
        // println!("{:?} {:?}", event.get_keycode(), event.get_keyval().name());
        use constants::*;
        #[allow(non_upper_case_globals)]
        Inhibit(match event.get_keyval() {
            Escape => {
                window.close();
                true
            }
            Up | Down | Page_Up | Page_Down | Tab | Shift_L | Shift_R | Control_L | Control_R
            | Alt_L | Alt_R | Return | ISO_Left_Tab => false,
            _ => {
                if !event.get_is_modifier() && !entry_clone.has_focus() {
                    entry_clone.grab_focus_without_selecting();
                }
                false
            }
        })
    });

    let matcher = SkimMatcherV2::default();
    entry.connect_changed(clone!(entries, listbox => move |e| {
        let text = e.get_text();
        {
            let mut entries = entries.borrow_mut();
            for entry in entries.values_mut() {
                entry.score = if text.is_empty() {
                    entry.label.set_attributes(None);
                    100
                } else if let Some((score, indices)) = matcher.fuzzy_indices(&entry.name, &text) {
                    let attr_list = pango::AttrList::new();

                    for i in indices {
                        let mut attr = pango::Attribute::new_background(65535, 0, 0).expect("Couldn't create new background");
                        let i = i as u32;
                        attr.set_start_index(i);
                        attr.set_end_index(i+1);
                        attr_list.insert(attr);
                    }

                    entry.label.set_attributes(Some(&attr_list));
                
                    score
                } else {
                    0
                };
            }
        }
        listbox.invalidate_filter();
        listbox.invalidate_sort()
    }));

    listbox.connect_row_activated(clone!(entries, window => move |_, r| {
        let e = entries.borrow();
        let context = gdk::Display::get_default().unwrap().get_app_launch_context().unwrap();
        e[r].info.launch(&[], Some(&context)).unwrap();
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
        if ea.score == eb.score {
            string_collate( &e[a].name, &e[b].name) as i32
        } else{
            eb.score.cmp(&ea.score) as i32
        }
    }))));

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
