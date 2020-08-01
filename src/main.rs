use std::path::Path;
use std::result::Result;
use std::vec::Vec;
use freedesktop_entry_parser::parse_entry;
use locale_settings::locale::{get_locale, set_locale_all_from_env, Category};
use locale_types::{Locale, LocaleString, LocaleIdentifier};
use std::str::FromStr;
use pathsearch::find_executable_in_path;
use std::fs::metadata;
use std::os::unix::fs::PermissionsExt;

#[derive(Debug)]
struct ApplicationEntry {
    name: String,
    comment: Option<String>,
    exec: String,
    icon: Option<String>
}

macro_rules! stry {
    ($e:expr, $o:path, $s:expr) => {
        match $e {
            $o(e) => e,
            _ => return Err($s)
        }
    };
}

impl ApplicationEntry {
    fn parse(input: impl AsRef<Path>, locales: &Vec<String>) -> Result<Self, &'static str> {
        let entry = stry!(parse_entry(input), Ok, "Parse failed");
        let section = entry.section("Desktop Entry");

        let get_attr = |name: &str| -> Option<&str> {
            for l in locales {
                if let attr@Some(_) = section.attr_with_param(name, l) {
                    return attr;
                }
            }
            section.attr(name)
        };

        let name = stry!(get_attr("Name"), Some, "Name missing");
        let comment = get_attr("Comment");
        let exec = stry!(section.attr("Exec"), Some, "Exec missing");

        if let Some(try_exec) = section.attr("TryExec") {
            let exec_path = stry!(find_executable_in_path(try_exec), Some, "TryExec file not found");
            let meta = stry!(metadata(exec_path), Ok, "Could not read TryExec metadata");
            if !meta.is_file() || meta.permissions().mode() & 0o111 == 0 {
                return Err("TryExec is not an executable file");
            }
        }

        let icon = get_attr("Icon");
        Ok(ApplicationEntry {
            name: name.to_string(),
            comment: comment.map(Into::into),
            exec: exec.to_string(),
            icon: icon.map(Into::into)
        })
    }
}

fn get_locale_strings(locale: &Locale) -> Vec<String> {
    let mut vec = Vec::new();
    if let Locale::String(s) = locale {
        let lang = s.language_code();
        if let (Some(c), Some(m)) = (s.territory(), s.modifier()) {
            vec.push(format!("{}_{}@{}", lang, c, m));
        }
        if let Some(c) = s.territory() {
            vec.push(format!("{}_{}", lang, c));
        }
        if let Some(m) = s.modifier() {
            vec.push(format!("{}@{}", lang, m));
        }
        vec.push(lang);
    }
    vec
}

fn main() -> Result<(), &'static str> { 
    set_locale_all_from_env();
    let loc = get_locale(&Category::Message).unwrap();
    let locs = get_locale_strings(&loc);
    println!("{:?}", locs);
    let entry = ApplicationEntry::parse("/usr/share/applications/org.kde.ark.desktop", &locs);
    println!("{:?}", entry);

    // loc = Locale::from_str("de_DE.UTF-8@test").unwrap();
    // println!("{:?}", get_locale_strings(&loc));

    println!("{:?}", find_executable_in_path("ls"));
    println!("{:?}", find_executable_in_path("/asdf/ls"));

    Ok(())
}
