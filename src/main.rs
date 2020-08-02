use libc::{LC_COLLATE, LC_MESSAGES};
use std::str::FromStr;

mod locale;
use locale::*;
mod app_entry;
use app_entry::*;

fn main() -> Result<(), &'static str> {
    set_locale(LC_MESSAGES, "");
    set_locale(LC_COLLATE, "");

    let locale = Locale::from_str(&get_locale(LC_MESSAGES).unwrap()).unwrap();
    let locale_strings = get_locale_strings(&locale);

    let mut entries = ApplicationEntry::parse_all(&locale_strings);
    entries.sort_by(|a, b| string_collate(&a.name, &b.name));

    for e in entries {
        println!("{:?}", e);
    }

    Ok(())
}
