use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};
use super::util::get_history_file;
use std::fs::File;

#[derive(Deserialize, Serialize)]
pub struct History {
    pub last_used: HashMap<String, u64>
}

impl History {
    pub fn load() -> History {
        match get_history_file(false) {
            Some(file) => {
                let config_str = std::fs::read_to_string(file).expect("Cannot read history file");
                toml::from_str(&config_str).expect("Cannot parse config: {}")
            },
            _ => History { last_used: HashMap::new() }
        }
    }

    pub fn save(&self) {
        let file = get_history_file(true).expect("Cannot create history file or cache directory");
        let mut file = File::create(file).expect("Cannot open history file for writing");
        let s = toml::to_string(self).unwrap();
        file.write_all(s.as_bytes()).expect("Cannot write to history file");
    }

    pub fn update(&mut self, id: &str) {
        let epoch = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
        self.last_used.insert(id.to_string(), epoch.as_secs());
    }
}