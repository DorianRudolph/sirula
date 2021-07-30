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
use crate::app_entry::AppEntry;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Write, BufRead, BufReader, BufWriter};
use std::process::Command;
use glib::{ObjectExt, GString, shell_parse_argv};
use std::path::PathBuf;
use gio::{AppInfo, AppInfoExt, AppInfoCreateFlags};
use gtk::{CssProvider, CssProviderExt};
use freedesktop_entry_parser::parse_entry;

pub fn get_xdg_dirs() -> xdg::BaseDirectories {
    xdg::BaseDirectories::with_prefix(APP_NAME).unwrap()
}

pub fn get_config_file(file: &str) -> Option<PathBuf> {
    get_xdg_dirs().find_config_file(file)
}

pub fn get_history_file(place: bool) -> Option<PathBuf>  {
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
            &gdk::Screen::get_default().expect("Error initializing gtk css provider."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );    
    }
}

pub fn load_history() -> Option<HashMap<String, usize>> {
    let mut file = BufReader::new(File::open(&get_history_file(false)?).ok()?);
    let (mut line, mut history) = (String::new(), HashMap::new());
    while file.read_line(&mut line).ok()? > 0 {
        if let Some((num, name)) = line.split_once(' ') {
            if let Ok(num) = num.parse::<usize>() {
                history.insert(name.trim_end().into(), num);
            }
        }
        line.clear()
    }
    Some(history)
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

pub fn store_history<'a, I>(entries: I, current: &str)
where I: Iterator<Item=&'a AppEntry> {
    let file = get_history_file(true).expect("Cannot create history file or cache directory");
    let file = File::create(file).expect("Cannot open history file for writing");
    let mut file = BufWriter::new(file);

    let write_error = "Cannot write to history file";
    entries
        .map(|e| (&e.display_string[..], e.usage + (e.display_string == current) as usize))
        .filter(|(_, u)| *u != 0)
        .for_each(|(n, u)| write!(&mut file, "{} {}\n", u, n).expect(write_error));

    file.flush().expect(write_error)
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
