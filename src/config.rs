use serde_derive::Deserialize;
use super::consts::*;
use super::util::get_config_file;

fn default_side() -> Side { Side::Right }
fn default_markup_highlight() -> String { "foreground=\"red\" underline=\"double\"".to_string() }
fn default_markup_exe() -> String { "font_style=\"italic\" font_size=\"smaller\"".to_string() }
fn default_exclusive() -> bool { true }

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Left,
    Right
}

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(default = "default_side")]
    pub side: Side,
    #[serde(default = "default_markup_highlight")]
    pub markup_highlight: String,
    #[serde(default = "default_markup_exe")]
    pub markup_exe: String,
    #[serde(default = "default_exclusive")]
    pub exclusive: bool
}

pub fn load_config() -> Config {
    let config_str = match get_config_file(CONFIG_FILE) {
        Some(file) => std::fs::read_to_string(file).expect("Cannot read config"),
        _ => "".to_owned()
    };
    let config: Config = toml::from_str(&config_str).expect("Cannot parse config: {}");
    config
}
