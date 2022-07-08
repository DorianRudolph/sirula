use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};
use super::util::get_history_file;
use std::fs::File;

#[derive(Copy, Clone, Default, Eq, Deserialize, Serialize)]
pub struct HistoryData {
    pub last_used : u64,
    pub usage_count : u32
}

impl PartialEq for HistoryData {
    fn eq(&self, other: &Self) -> bool {
        self.last_used.eq(&other.last_used) && self.usage_count.eq(&other.usage_count)
    }
}

pub fn load_history() -> HashMap<String, HistoryData> {
    match get_history_file(false) {
        Some(file) => {
            let history_str = std::fs::read_to_string(file).expect("Cannot read history file");
            toml::from_str(&history_str).unwrap_or_else(|err| {
                eprintln!("Cannot parse history file: {}", err);
                HashMap::new()
            })
        },
        _ => HashMap::new()
    }
}

pub fn save_history(history: &HashMap<String, HistoryData>) {
    let file = get_history_file(true).expect("Cannot create history file or cache directory");
    let mut file = File::create(file).expect("Cannot open history file for writing");
    let s = toml::to_string(history).unwrap();
    file.write_all(s.as_bytes()).expect("Cannot write to history file");
}

pub fn update_history(history: &mut HashMap<String, HistoryData>, id: &str) {
    let epoch = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
    let usage_count;
    match history.get(&id.to_string()) {
        Some(history_match) => { usage_count = history_match.usage_count + 1 },
        None => { usage_count = 1 }
    }

    history.insert( id.to_string(), HistoryData{ last_used: epoch.as_secs(), usage_count } );
}
