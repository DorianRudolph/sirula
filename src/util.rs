use super::consts::*;
use glib::{ObjectExt, GString};
use std::path::PathBuf;
use gio::{AppInfo, AppInfoExt, AppInfoCreateFlags};
use gtk::{CssProvider, CssProviderExt};
use freedesktop_entry_parser::parse_entry;

pub fn get_config_file(file: &str) -> Option<PathBuf> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix(APP_NAME).unwrap();
    xdg_dirs.find_config_file(file)
}

pub fn load_css() {
    if let Some(file) = get_config_file(STYLE_FILE) {
        let provider = CssProvider::new();
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

pub fn launch_app(info: &AppInfo) {
    let context = gdk::Display::get_default().unwrap().get_app_launch_context().unwrap();

    // launch terminal applications ourselves because GTK ignores the TERMINAL environment variable
    if let Some(term) = std::env::var_os("TERMINAL") {
        let use_terminal = info.get_property("filename").ok().and_then(|p| p.get::<GString>().ok()).flatten()
            .and_then(|s| parse_entry(s.to_string()).ok())
            .and_then(|e| e.section("Desktop Entry").attr("Terminal").map(|t| t == "1" || t == "true"))
            .unwrap_or(false);
        if use_terminal {
            if let Some(command) = info.get_commandline().or(info.get_executable()) {
                let mut cmd_line = term;
                cmd_line.push(" -e ");
                cmd_line.push(command);
                if let Ok(info) = AppInfo::create_from_commandline(cmd_line, None, AppInfoCreateFlags::NONE) {
                    info.launch(&[], Some(&context)).expect("Error while launching terminal app");
                    return;
                }
            }
        }
    }

    info.launch(&[], Some(&context)).expect("Error while launching terminal app");
}

#[macro_export]
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
