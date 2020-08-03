use freedesktop_entry_parser::parse_entry;
use pathsearch::find_executable_in_path;
use std::{
    collections::HashMap, fs, fs::metadata, os::unix::fs::PermissionsExt, path::Path,
    result::Result, vec::Vec,
};
use shellexpand;

#[derive(Debug)]
pub struct ApplicationEntry {
    pub name: String,
    pub generic_name: Option<String>,
    pub comment: Option<String>,
    pub exec: String,
    pub terminal: bool,
    pub icon: Option<String>,
}

macro_rules! stry {
    ($e:expr, $o:path, $s:expr) => {
        match $e {
            $o(e) => e,
            _ => return Err($s),
        }
    };
}

impl ApplicationEntry {
    pub fn parse(input: impl AsRef<Path>, locales: &Vec<String>) -> Result<Self, &'static str> {
        let entry = stry!(parse_entry(input), Ok, "Parse failed");
        let section = entry.section("Desktop Entry");

        let get_attr = |name: &str| -> Option<&str> {
            for l in locales {
                if let attr @ Some(_) = section.attr_with_param(name, l) {
                    return attr;
                }
            }
            section.attr(name)
        };

        let name = stry!(get_attr("Name"), Some, "Name missing");
        let generic_name = get_attr("GenericName");
        let comment = get_attr("Comment");
        let exec = stry!(section.attr("Exec"), Some, "Exec missing");
        let terminal = match section.attr("Terminal") {
            Some("1") | Some("true") => true,
            _ => false,
        };

        if let Some("1") | Some("true") = section.attr("NoDisplay") {
            return Err("NoDisplay is set");
        }

        if let Some("1") | Some("true") = section.attr("Hidden") {
            return Err("Hidden is set");
        }

        if let Some(try_exec) = section.attr("TryExec") {
            let exec_path = stry!(
                find_executable_in_path(try_exec),
                Some,
                "TryExec file not found"
            );
            let meta = stry!(metadata(exec_path), Ok, "Could not read TryExec metadata");
            if !meta.is_file() || meta.permissions().mode() & 0o111 == 0 {
                return Err("TryExec is not an executable file");
            }
        }

        let icon = get_attr("Icon");
        Ok(ApplicationEntry {
            name: name.to_string(),
            generic_name: generic_name.map(Into::into),
            comment: comment.map(Into::into),
            exec: exec.to_string(),
            terminal: terminal,
            icon: icon.map(Into::into),
        })
    }

    const LOCATIONS: &'static [&'static str] = &[
        "/usr/share/applications/",
        "/usr/local/share/applications/",
        "~/.local/share/applications/",
    ];

    pub fn parse_all(locale_strings: &Vec<String>) -> Vec<ApplicationEntry> {
        let mut app_entries = HashMap::new();
        for loc in Self::LOCATIONS {
            let loc_expanded = shellexpand::tilde(loc).to_string();
            if let Ok(dir) = fs::read_dir(loc_expanded) {
                for entry in dir {
                    if let Ok(e) = entry {
                        if e.path().is_file() {
                            if let Ok(ae) = Self::parse(e.path(), &locale_strings) {
                                app_entries.insert(e.file_name(), ae);
                            }
                        }
                    }
                }
            }
        }
        app_entries.drain().map(|(_, ae)| ae).collect()
    }
}
