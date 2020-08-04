use super::consts::*;
use std::path::PathBuf;
use gtk::{CssProvider,CssProviderExt};

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
