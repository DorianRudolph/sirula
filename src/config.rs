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
use super::util::get_config_file;
use pango::Attribute;
use serde::{de::Error, Deserializer};
use serde_derive::Deserialize;
use std::collections::HashMap;

macro_rules! make_config {
    ($name:ident { $($field:ident : $type:ty $( = ($default:expr) $field_str:literal )? $( [$serde_opts:expr])? ),* }) => {
        #[derive(Deserialize, Debug)]
        pub struct $name { $(
            #[serde( $(default = $field_str )? )]
            $(#[serde($serde_opts)])?
            pub $field: $type,
        )* }
        $( $( fn $field() -> $type { $default } )? )*
    };
}

#[derive(Deserialize, Debug, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Field {
    Comment,
    Id,
    IdSuffix,
    Executable,
    Commandline,
}

// not sure how to avoid having to specify the name twice
make_config!(Config {
    markup_default: Vec<Attribute> = (Vec::new()) "markup_default" [deserialize_with = "deserialize_markup"],
    markup_highlight: Vec<Attribute> = (parse_attributes("foreground=\"red\" underline=\"double\"").unwrap()) "markup_highlight" [deserialize_with = "deserialize_markup"],
    markup_extra: Vec<Attribute> = (parse_attributes("font_style=\"italic\" font_size=\"smaller\"").unwrap()) "markup_extra" [deserialize_with = "deserialize_markup"],
    exclusive: bool = (true) "exclusive",
    frequent_first: bool = (false) "frequent_first",
    recent_first: bool = (true) "recent_first",
    prune_history: u32 = (0) "prune_history",
    icon_size: i32 = (64) "icon_size",
    lines: i32 = (2) "lines",
    margin_left: i32 = (0) "margin_left",
    margin_right: i32 = (0) "margin_right",
    margin_top: i32 = (0) "margin_top",
    margin_bottom: i32 = (0) "margin_bottom",
    anchor_left: bool = (false) "anchor_left",
    anchor_right: bool = (true) "anchor_right",
    anchor_top: bool = (true) "anchor_top",
    anchor_bottom: bool = (true) "anchor_bottom",
    width: i32 = (-1) "width",
    height: i32 = (-1) "height",
    extra_field: Vec<Field> = (vec![Field::IdSuffix]) "extra_field",
    hidden_fields: Vec<Field> = (Vec::new()) "hidden_fields",
    name_overrides: HashMap<String, String> = (HashMap::new()) "name_overrides",
    hide_extra_if_contained: bool = (true) "hide_extra_if_contained",
    command_prefix: String = (":".into()) "command_prefix",
    exclude: Vec<String> = (Vec::new()) "exclude",
    term_command: Option<String> = (None) "term_command"
});

fn deserialize_markup<'de, D>(deserializer: D) -> Result<Vec<Attribute>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = serde::Deserialize::deserialize(deserializer)?;
    parse_attributes(s).map_err(D::Error::custom)
}

impl Config {
    pub fn load() -> Config {
        let config_str = match get_config_file(CONFIG_FILE) {
            Some(file) => std::fs::read_to_string(file).expect("Cannot read config"),
            _ => "".to_owned(),
        };
        let config: Config = toml::from_str(&config_str).expect("Cannot parse config: {}");
        config
    }
}

fn parse_attributes(markup: &str) -> Result<Vec<Attribute>, String> {
    let (attributes, _, _) = pango::parse_markup(&format!("<span {}>X</span>", markup), '\0')
        .map_err(|err| format!("Failed to parse markup: {}", err))?;
    let mut iter = attributes.iterator().ok_or("Failed to parse markup")?;
    Ok(iter.attrs())
}
