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

use super::Config;
use crate::consts::*;
use freedesktop_entry_parser::parse_entry;
use gio::{prelude::AppInfoExt, AppInfo};
use glib::{shell_parse_argv, GString, ObjectExt};
use gtk::{prelude::CssProviderExt, CssProvider};
use std::path::PathBuf;
use std::process::{id, Command};

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

pub fn launch_app(info: &AppInfo, term_command: Option<&str>, launch_cgroups: bool) {
    let mut command: String = info
        .commandline()
        .unwrap_or_else(|| info.executable())
        .to_str()
        .unwrap()
        .to_string()
        .replace("%U", "")
        .replace("%F", "")
        .replace("%u", "")
        .replace("%f", "");

    if info
        .try_property::<GString>("filename")
        .ok()
        .and_then(|s| parse_entry(&s).ok())
        .and_then(|e| {
            e.section("Desktop Entry")
                .attr("Terminal")
                .map(|t| t == "1" || t == "true")
        })
        .unwrap_or_default()
    {
        command = if let Some(term) = term_command {
            term.to_string().replace("{}", &command)
        } else if let Some(mut term) = std::env::var_os("TERMINAL") {
            term.push(" -e ");
            term.push(command);
            term.into_string().expect("couldn't convert to string")
        } else {
            return;
        };
    }
    if launch_cgroups {
        let mut name = info.id().unwrap().to_string();
        name.truncate(name.len() - 8); // remove .desktop extension
        let parsed = Command::new("systemd-escape")
            .arg(name)
            .output()
            .unwrap()
            .stdout;
        command = format!(
            "systemd-run --scope --user --unit=app-sirula-{}-{} {}",
            String::from_utf8_lossy(&parsed).trim(),
            id(),
            command
        );
    }

    let command = command.split_whitespace().collect::<Vec<_>>();
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
