use libc::LC_ALL;
mod locale;
use locale::*;
use gdk::keys::constants;
use gio::prelude::*;
use gtk::prelude::*;
use gtk::{
    BoxBuilder, IconLookupFlags, IconTheme, IconThemeExt, ImageBuilder, Label, LabelExt,
    ListBoxRow, Orientation, WidgetExt
};
use pango::EllipsizeMode;
use gio::AppInfo;
use std::env::args;
use log::LevelFilter;
use std::{cell::RefCell, collections::HashMap, io::Write, rc::Rc, path::PathBuf, fs};
use glib::shell_unquote;
use futures::prelude::*;
use log::{error, debug};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use serde_derive::Deserialize;

const APP_ID: &str = "com.dorian.sirula";
const APP_NAME: &str = "sirula";

const STYLE_FILE: &str = "style.css";
const CONFIG_FILE: &str = "config.toml";

const APP_LABEL_CLASS: &str = "app-label";
const APP_ICON_CLASS: &str = "app-icon";
const APP_ROW_CLASS: &str = "app-row";
const ROOT_BOX_NAME: &str = "root-box";
const LISTBOX_NAME: &str = "app-list";
const SEARCH_ENTRY_NAME: &str = "search";
const SCROLL_NAME: &str = "scroll";

fn default_side() -> Side { Side::Right }
fn default_markup_highlight() -> String { "underline=\"single\"".to_string() }
fn default_markup_exe() -> String { "font_style=\"italic\" font_size=\"smaller\"".to_string() }
fn default_exclusive() -> bool { true }

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Side {
    Left,
    Right
}

#[derive(Deserialize, Debug)]
struct Config {
    #[serde(default = "default_side")]
    side: Side,
    #[serde(default = "default_markup_highlight")]
    markup_highlight: String,
    #[serde(default = "default_markup_exe")]
    markup_exe: String,
    #[serde(default = "default_exclusive")]
    exclusive: bool
}

fn load_config() -> Config {
    let config_str = match get_config_file(CONFIG_FILE) {
        Some(file) => fs::read_to_string(file).expect("Cannot read config"),
        _ => "".to_owned()
    };
    let config: Config = toml::from_str(&config_str).expect("Cannot parse config: {}");
    debug!("Load config {:?}", config);
    config
}

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

        let label = gtk::LabelBuilder::new()
            .xalign(0.0f32)
            .label(&name)
            .wrap(true)
            .ellipsize(EllipsizeMode::End)
            .lines(2)
            .build();
        label.get_style_context().add_class(APP_LABEL_CLASS);

        if let Some(filename) = app.get_executable().and_then(|p| p.file_name().map(|f| f.to_owned())) {
            if let Ok(filename) = shell_unquote(filename) {
                let filename = filename.to_string_lossy();
                if !name.to_lowercase().contains(&filename.to_lowercase()) {
                    label.set_markup(&format!("{} <small><i>{}</i></small>", name, filename));
                    name = format!("{} {}", name, filename);
                }
            }
        }

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
    entries
}

fn get_config_file(file: &str) -> Option<PathBuf> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix(APP_NAME).unwrap();
    xdg_dirs.find_config_file(file)
}

fn load_css() {
    if let Some(file) = get_config_file(STYLE_FILE) {
        let provider = gtk::CssProvider::new();
        if let Err(err) = provider.load_from_path(file.to_str().unwrap()) {
            eprintln!("Failed to load CSS: {}", err);
        }
        gtk::StyleContext::add_provider_for_screen(
            &gdk::Screen::get_default().expect("Error initializing gtk css provider."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );    
    }
}

fn activate(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);

    let config = load_config();

    gtk_layer_shell::init_for_window(&window);
    gtk_layer_shell::set_keyboard_interactivity(&window, true);
    gtk_layer_shell::set_layer(&window, gtk_layer_shell::Layer::Overlay);
    gtk_layer_shell::auto_exclusive_zone_enable(&window);

    // gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Left, 10);
    // gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Right, 10);
    // gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Top, 10);

    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Left, config.side == Side::Left);
    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Right, config.side != Side::Left);
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

    let entries = Rc::new(RefCell::new(load_entries()));

    for (row, _) in &entries.borrow() as &HashMap<ListBoxRow, AppEntry> {
        listbox.add(row);
    }

    window.connect_key_press_event(clone!(entry => move |window, event| {
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
                if !event.get_is_modifier() && !entry.has_focus() {
                    entry.grab_focus_without_selecting();
                }
                false
            }
        })
    }));

    let matcher = SkimMatcherV2::default();
    entry.connect_changed(clone!(entries, listbox => move |e| {
        let mut has_matches = false;
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
                if entry.score > 0 {
                    has_matches = true;
                }
            }
        }
        listbox.invalidate_filter();
        listbox.invalidate_sort();
        if has_matches {
            let row = listbox.get_row_at_index(0);
            listbox.select_row(row.as_ref());
        } else {
            listbox.select_row::<ListBoxRow>(None);
        }
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
        (if ea.score == eb.score {
            string_collate( &e[a].name, &e[b].name)
        } else {
            eb.score.cmp(&ea.score)
        }) as i32
    }))));

    window.add(&vbox);
    window.show_all()
}

fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder
        .format(|buf, record| {
            writeln!(buf, "{} | {} | {}", buf.timestamp_millis(),
                record.level(), record.args()
            )
        })
        .filter(None, LevelFilter::Debug)
        .init();

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
