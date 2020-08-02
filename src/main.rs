use std::{
    cmp::Ordering, path::Path, result::Result, vec::Vec, collections::HashMap,
    fs, os::unix::fs::PermissionsExt, fs::metadata, str::FromStr, ffi::{CStr, CString},
    ptr};
use freedesktop_entry_parser::parse_entry;
use locale_types::{Locale, LocaleIdentifier};
use pathsearch::find_executable_in_path;
use libc::{strcoll, setlocale, LC_COLLATE, LC_MESSAGES};

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
        
        if let Some("1") | Some("true") = section.attr("NoDisplay") {
            return Err("NoDisplay is set");
        }

        if let Some("1") | Some("true") = section.attr("Hidden") {
            return Err("Hidden is set");
        }

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

    const LOCATIONS: &'static[&'static str] = &["/usr/share/applications/", "/usr/local/share/applications/", "~/.local/share/applications/"];

    fn parse_all(locales: &Vec<String>) -> Vec<ApplicationEntry> {
        let mut app_entries = HashMap::new();
        for loc in Self::LOCATIONS {
            if let Ok(dir) = fs::read_dir(loc) {
                for entry in dir {
                    if let Ok(e) = entry {
                        if e.path().is_file() {
                            if let Ok(ae) = Self::parse(e.path(), locales) {
                                app_entries.insert(e.file_name(), ae);
                            }
                        }
                    }
                }
            }
        }
        app_entries.drain().map(|(_,ae)| ae).collect()
    }
}

fn string_compare(a: &str, b: &str) -> Ordering {
    // Note: Only works properly if locale is set to UTF-8
    let ord = unsafe {
        let ac = CString::new(a).unwrap();
        let bc = CString::new(b).unwrap();
        strcoll(ac.as_ptr(), bc.as_ptr())
    };
    ord.cmp(&0)
}

fn set_locale(cat: i32, loc: &str) -> Option<String> {
    unsafe {
        let loc = CString::new(loc).unwrap();
        let c_str = setlocale(cat, loc.as_ptr());
        if c_str.is_null() {
            None
        } else {
            Some(CStr::from_ptr(c_str).to_string_lossy().to_string())
        }
    }
}

fn get_locale(cat: i32) -> Option<String> {
    unsafe {
        let c_str = setlocale(cat, ptr::null());
        if c_str.is_null() {
            None
        } else {
            Some(CStr::from_ptr(c_str).to_string_lossy().to_string())
        }
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
    println!("{:?}", set_locale(LC_COLLATE, ""));
    println!("{:?}", set_locale(LC_MESSAGES, ""));
    let loc = Locale::from_str(&get_locale(LC_MESSAGES).unwrap()).unwrap();

    // println!("{:?}", get_locale(&Category::StringCollation));
    // set_locale_from_env(&Category::StringCollation);
    // println!("{:?}", get_locale(&Category::StringCollation));

    // let loc = get_locale(&Category::Message).unwrap();
    // let locs = get_locale_strings(&loc);

    // println!("{:?}", locs);
    // let entry = ApplicationEntry::parse("/usr/share/applications/org.kde.ark.desktop", &locs);
    // println!("{:?}", entry);

    // loc = Locale::from_str("de_DE.UTF-8@test").unwrap();
    // println!("{:?}", get_locale_strings(&loc));

    // println!("{:?}", find_executable_in_path("ls"));
    // println!("{:?}", find_executable_in_path("/asdf/ls"));

    let mut arr = vec!["ä".to_string(), "O".to_string(), "z".to_string(), "a".to_string(), "A".to_string(), "ö".to_string(), "Z".to_string(), "G".to_string(), "g".to_string(), "0".to_string()];
    arr.sort();
    println!("{:?}", arr);

    arr.sort_by(|a,b| string_compare(a, b));
    println!("{:?}", arr);

    arr.sort_by(|a,b| string_compare(a, b));
    println!("{:?}", arr);

    // set_locale(&Locale::from_str("en_US.UTF-8").unwrap(), &Category::StringCollation);
    // arr.sort_by(|a,b| string_compare(a, b));
    // println!("{:?}", arr);


    Ok(())
}
