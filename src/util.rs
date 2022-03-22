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

use crate::consts::*;
use freedesktop_entry_parser::parse_entry;
use gio::{
    prelude::{AppInfoExt, AppInfoExtManual},
    AppInfo, AppInfoCreateFlags,
};
use glib::{shell_parse_argv, GString, MainContext, ObjectExt};
use gtk::{prelude::CssProviderExt, CssProvider};
use osstrtools::OsStrTools;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;

pub fn get_xdg_dirs() -> xdg::BaseDirectories {
    xdg::BaseDirectories::with_prefix(APP_NAME).unwrap()
}

pub fn get_config_file(file: &str) -> Option<PathBuf> {
    get_xdg_dirs().find_config_file(file)
}

pub fn get_history_file(place: bool) -> Option<PathBuf> {
    let xdg = get_xdg_dirs();
    if place {
        xdg.place_cache_file(HISTORY_FILE).ok()
    } else {
        xdg.find_cache_file(HISTORY_FILE)
    }
}

pub fn load_css() {
    if let Some(file) = get_config_file(STYLE_FILE) {
        let provider = CssProvider::new();
        if let Err(err) = provider.load_from_path(file.to_str().unwrap()) {
            eprintln!("Failed to load CSS: {}", err);
        }
        gtk::StyleContext::add_provider_for_screen(
            &gdk::Screen::default().expect("Error initializing gtk css provider."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

pub fn is_cmd(text: &str, cmd_prefix: &str) -> bool {
    !cmd_prefix.is_empty() && text.starts_with(cmd_prefix)
}

pub fn launch_cmd(cmd_line: &str) {
    let mut parts = shell_parse_argv(cmd_line).expect("Error parsing command line");
    let mut parts_iter = parts.iter_mut();

    let cmd = parts_iter.next().expect("Expected command");

    let mut child = Command::new(cmd);
    child.args(parts_iter);
    child.spawn().expect("Error spawning command");
}

pub fn launch_app(info: &AppInfo, term_command: Option<&str>) {
    let context = gdk::Display::default()
        .unwrap()
        .app_launch_context()
        .unwrap();

    let condition = info
        .property("filename")
        .ok()
        .and_then(|p| p.get::<GString>().ok())
        .and_then(|s| parse_entry(s.to_string()).ok())
        .and_then(|e| {
            e.section("Desktop Entry")
                .attr("Terminal")
                .map(|t| t == "1" || t == "true")
        })
        .unwrap_or_default();

    if condition {
        let command = (match info.commandline() {
            Some(c) => c,
            _ => info.executable(),
        })
        .as_os_str()
        .quote_single();

        let commandline = if let Some(term) = term_command {
            OsStr::new(term).replace("{}", command)
        } else if let Some(mut term) = std::env::var_os("TERMINAL") {
            term.push(" -e ");
            term.push(command);
            term
        } else {
            return;
        };

        let info = AppInfo::create_from_commandline(commandline, None, AppInfoCreateFlags::NONE)
            .expect("Failed to create AppInfo from commandline");
        info.launch(&[], Some(&context))
            .expect("Error while launching terminal app");
    } else {
        let future = info.launch_uris_async_future(&[], Some(&context));
        MainContext::default()
            .block_on(future)
            .expect("Error while launching app");
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
