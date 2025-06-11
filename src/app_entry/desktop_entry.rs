use freedesktop_desktop_entry::{
	default_paths, get_languages_from_env, Iter, Group
};
use which::which;

use std::{
	path::PathBuf,
	env::var,
};

pub struct DesktopEntry {
	pub id: String,
	pub path: PathBuf,
	pub name: String,
	pub desc: String,
	pub icon: Option<String>,
	pub exec: String,
	pub terminal: bool,
	pub prefers_nondefault_gpu: bool,
	pub actions: Vec<DesktopAction>,
}

pub struct DesktopAction {
	pub name: String,
	pub exec: String,
}

macro_rules! skip_none { // TODO: add id
    ($res:expr, $id:expr) => {
        match $res {
            Some(val) => val,
            None => {
            	println!("skiping {}", $id);
                continue;
            }
        }
    };
}

impl DesktopEntry {
	pub fn get() -> Vec<DesktopEntry> {
	    let locales = get_languages_from_env();
	    let entries = Iter::new(default_paths())
	        .entries(Some(&locales))
	        .collect::<Vec<_>>();

	    let mut out = Vec::new();

		for entry in entries {
			let id = entry.appid;
			let desktop_entry = entry.groups.0.get("Desktop Entry").unwrap();
			
			{ // skip if conditions are met
				let hidden = get_key_bool(desktop_entry, "Hidden").unwrap_or_default();
				let nodisplay = get_key_bool(desktop_entry, "NoDisplay").unwrap_or_default();

				let only_show_in_str = get_key(desktop_entry, "OnlyShowIn");
				let not_show_in_str = get_key(desktop_entry, "NotShowIn");
				let mut only_show_in = false;
				let mut not_show_in = false;

				match var("XDG_CURRENT_DESKTOP") {
					Ok(x) => {
						if let Some(strx) = only_show_in_str {
							only_show_in = ! strx.contains(&x)
						}
						if let Some(strx) = not_show_in_str {
							not_show_in = strx.contains(&x)
						}
					},
					Err(_) => {
						println!("XDG_CURRENT_DESKTOP env variable isn't set!");
						only_show_in = only_show_in_str.is_some();
					}
				};
				if not_show_in || only_show_in || hidden || nodisplay {
					println!("skiping: {}", &id);
					continue
				}
			}
			let mut actions = Vec::new();
			
			for desktop_action in entry.groups.0.iter() {
				if desktop_action.0.starts_with("Desktop Action ") {
	   				let action = desktop_action.1;
	   				actions.push(DesktopAction {
						name: skip_none!(get_key(action, "Name"), id),
						exec: skip_none!(get_exec_key(action), id),
					})
				}
			}
			
			let app_entry = DesktopEntry {
				path: entry.path,
				name: skip_none!(get_key(desktop_entry, "Name"), id),
				desc: get_key(desktop_entry, "Description").unwrap_or_default(),
				icon: get_key(desktop_entry, "Icon"),
				exec: skip_none!(get_exec_key(desktop_entry), id),
				terminal: get_key_bool(desktop_entry, "Terminal").unwrap_or_default(),
				prefers_nondefault_gpu: get_key_bool(desktop_entry, "PrefersNonDefaultGPU").unwrap_or_default(),
				id, actions,
			};

			out.push(app_entry)
		}
		out
	}
}

fn get_exec_key(group: &Group) -> Option<String> {
	match get_key(group, "TryExec") {
		Some(try_exec) => {
			match which(&try_exec) {
				Ok(_) => Some(try_exec),
				Err(_) => get_key(group, "Exec"),
			}
		}
		None => {
			get_key(group, "Exec")
		}
	}
}

fn get_key_bool(group: &Group, key: &str) -> Option<bool> {
	let string = get_key(group, key)?;
	match string.as_str() {
		"true" => Some(true),
		"false" => Some(false),
		_ => None,
	}
}

fn get_key(group: &Group, key: &str) -> Option<String> {
	match group.0.get(key) {
		Some(x) => {
			match x.1.clone().into_values().next() {
				Some(x) => Some(x),
				None => Some(x.0.clone()),
			}
		},
		None => None,
	}
}

impl PartialEq for DesktopEntry {
	fn eq(&self, other: &Self) -> bool {
		self.id == other.id
	}
}

impl Eq for DesktopEntry {}
