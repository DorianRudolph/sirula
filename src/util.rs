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

use super::consts::*;
use std::env::{var, VarError};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Write, BufRead, BufReader, BufWriter};
use std::process::Command;
use glib::{ObjectExt, GString, shell_parse_argv};
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

pub fn get_recents_path() -> Result<PathBuf, VarError> {
    let mut file = match var(r"XDG_CACHE_HOME") {
        Ok(file) => file.into(),
        Err(_) => {
            let mut home = PathBuf::from(var(r"HOME")?);
            home.push(r".cache");
            home
        }
    };
    file.push(r"sirula-recents");
    Ok(file)
}

pub fn load_recents() -> Option<HashMap<String, usize>> {
    let mut file = BufReader::new(File::open(&get_recents_path().ok()?).ok()?);
    let (mut line, mut recents) = (String::new(), HashMap::new());
    while file.read_line(&mut line).ok()? > 0 {
        if let Some((num, name)) = line.split_once(' ') {
            if let Ok(num) = num.parse::<usize>() {
                recents.insert(name.trim_end().into(), num);
            }
        }
        line.clear()
    }
    Some(recents)
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

pub fn store_recents(recents: &HashMap<String, usize>, mut current: &str) {
    let file = get_recents_path().expect("Error reading variable");
    let file = File::create(file).expect("Cannot open recents file for writing");
    let mut file = BufWriter::new(file);

    let write_error = "Cannot write to recents file";
    for (name, &(mut num)) in recents {
        if name == current {
            current = "";
            num += 1;
        }
        write!(&mut file, "{} {}\n", num, name).expect(write_error);
    }

    if !current.is_empty() {
        write!(&mut file, "1 {}\n", current).expect(write_error);
    }

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
