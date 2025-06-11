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
use gio::{prelude::AppInfoExt, AppInfo};
use glib::{shell_parse_argv, GString, ObjectExt};
use gtk::{prelude::CssProviderExt, CssProvider};
use std::path::PathBuf;
use std::process::{id, Command};
use shlex::Shlex;
use crate::app_entry::desktop_entry::DesktopEntry;

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

pub fn launch_app(info: &DesktopEntry, term_command: Option<&str>, launch_cgroups: bool) {
    let command_string = info
        .exec
        .replace("%U", "")
        .replace("%F", "")
        .replace("%u", "")
        .replace("%f", "");
    let mut command: Vec<String> = Shlex::new(&command_string).collect();

    if info.terminal
    {
        if let Some(term) = term_command {
            let command_string = term.to_string().replace("{}", &command_string);
		    command = Shlex::new(&command_string).collect();
        } else if let Some(term) = std::env::var_os("TERMINAL") {
        	let term = term.into_string().expect("couldn't convert to string");
        	let mut command_new = vec![term, "-e".into()];
        	command_new.extend(command);
        	command = command_new;
        } else {
            return;
        };
    }
    if launch_cgroups { // TODO: clone
        // info.id.clone().truncate(info.id.len() - ".desktop".len()); // remove .desktop extension
        let parsed = Command::new("systemd-escape")
            .arg(&info.id)
            .output()
            .unwrap()
            .stdout;
        let unit = format!(
            "--unit=app-sirula-{}-{}",
            String::from_utf8_lossy(&parsed).trim(),
            id()
        );
        let mut command_new: Vec<String> = vec!["systemd-run".into(), "--scope".into(), "--user".into(), unit];
        command_new.extend(command);
        command = command_new;
    }

    Command::new(&command[0])
        .args(&command[1..])
        .spawn()
        .expect("Error launching app");
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
