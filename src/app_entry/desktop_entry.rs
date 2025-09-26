use freedesktop_desktop_entry::{default_paths, get_languages_from_env, Group, Iter};
use log::{info, error, warn};
use which::which;
use shlex::Shlex;

use std::{
	collections::HashMap,
	env::var,
	path::PathBuf,
    process::{id, Command, Child},
    error::Error,
};
pub struct DesktopEntry {
    pub id: String,
    #[allow(dead_code)]
    pub file: PathBuf,
    pub name: String,
    pub exec: String,

    pub generic_name: Option<String>,
    pub keywords: Option<String>,
    pub comment: Option<String>,
    pub categories: Option<String>,

    pub icon: Option<String>,

    pub path: Option<String>,
    pub terminal: bool,
    pub prefers_nondefault_gpu: bool,

    #[allow(dead_code)]
    pub actions: Vec<DesktopAction>,
}

#[allow(dead_code)]
pub struct DesktopAction {
	pub id: String,
    pub name: String,
    pub exec: String,
}

macro_rules! skip_none {
    ($res:expr, $id:expr) => {
        match $res {
            Some(val) => val,
            None => {
                error!("skipping: {} (missing/wrong values)", $id);
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

        let mut out = HashMap::new();
        let xdg_current_desktop = var("XDG_CURRENT_DESKTOP");
        if let Err(e) = &xdg_current_desktop {
            warn!("XDG_CURRENT_DESKTOP env variable can't be read! {e}");
        }

        for entry in entries.into_iter().rev() {
            let id = entry.appid;
            let desktop_entry = entry.groups.0.get("Desktop Entry").unwrap();

            {
                // skip if conditions are met
                let hidden = get_key_bool(desktop_entry, "Hidden").unwrap_or_default();
                let nodisplay = get_key_bool(desktop_entry, "NoDisplay").unwrap_or_default();

                let only_show_in_str = get_key(desktop_entry, "OnlyShowIn");
                let not_show_in_str = get_key(desktop_entry, "NotShowIn");
                let mut only_show_in = false;
                let mut not_show_in = false;

                match &xdg_current_desktop {
                    Ok(x) => {
                        if let Some(strx) = only_show_in_str {
                            only_show_in = !strx.contains(x)
                        }
                        if let Some(strx) = not_show_in_str {
                            not_show_in = strx.contains(x)
                        }
                    }
                    Err(_) => {
                        only_show_in = only_show_in_str.is_some();
                    }
                };
                if not_show_in || only_show_in || hidden || nodisplay {
                    info!("skipping: {} (hidden)", &id);
                    continue;
                }
            }
            let mut actions = Vec::new();

           	if let Some(strx) = get_key(desktop_entry, "Actions") {
            	let fields: Vec<&str> = strx.split(';').collect();
            	for field in fields {
            		if let Some(desktop_action) = entry.groups.0.iter().find(|x| x.0 == &format!("Desktop Action {field}")) {
	                    let action = desktop_action.1;
	                    actions.push(DesktopAction {
	                    	id: field.to_string(),
	                    	name: skip_none!(get_key(action, "Name"), id),
	                    	exec: skip_none!(get_exec_key(action), id),
	                    });
	            	}
            	}
            }

            let app_entry = DesktopEntry {
                id: id.clone(), // TODO: clone
                file: entry.path,
                name: skip_none!(get_key(desktop_entry, "Name"), id),
                exec: skip_none!(get_exec_key(desktop_entry), id),

                generic_name: get_key(desktop_entry, "GenericName"),
                comment: get_key(desktop_entry, "Comment"),
                keywords: get_key(desktop_entry, "Keywords"),
                categories: get_key(desktop_entry, "Categories"),

                icon: get_key(desktop_entry, "Icon"),

                path: get_key(desktop_entry, "Path"),
                terminal: get_key_bool(desktop_entry, "Terminal").unwrap_or_default(),
                prefers_nondefault_gpu: get_key_bool(desktop_entry, "PrefersNonDefaultGPU")
                    .unwrap_or_default(),

                actions,
            };

            if let Some(app_entry) = out.insert(id, app_entry) {
                info!("skipping: {} (overwritten)", app_entry.id)
            }
        }
        out.into_values().collect()
    }
    // pub fn get_command(x: Option<String>) -> Vec<String> {}
    pub fn launch(&self,
    	child: Option<String>,
	    term_command: Option<&str>,
	    launch_cgroups: bool,
	    gpu_variable: Option<String>,
	) -> Result<Child, Box<dyn std::error::Error>> {
	    let replace_keys = [
	        ("%U", ""), // link(s)
	        ("%u", ""),
	        ("%F", ""), // files(s)
	        ("%f", ""),
	        ("%D", ""), // Deprecated
	        ("%d", ""),
	        ("%N", ""),
	        ("%n", ""),
	        ("%v", ""),
	        ("%m", ""),
	        ("%i", &self.icon.clone().unwrap_or_default()), // icon
	        ("%c", &self.name),                             // name (translated)
	        ("%k", self.file.to_str().unwrap_or_default()), // filename as uri > file > none
	    ];

	    let mut command_string = if let Some(child) = child {
	    	let mut out = None;
	    	for action in &self.actions {
	    		if action.id == child {
	    			out = Some(action.exec.clone())
	    		}
	    	}
	   		if let Some(exec) = out {
	   			exec
	   		} else {
	   			return Err(format!("unknown action \"{child}\"").into())
	   		}
	    } else {
	    	self.exec.clone()
	    };
	    for replace_key in replace_keys {
	        command_string = command_string.replace(replace_key.0, replace_key.1)
	    }
	    let mut command: Vec<String> = Shlex::new(&command_string).collect();

	    if self.terminal {
	        if let Some(term) = term_command {
	            let command_string = term.to_string().replace("{}", &command_string);
	            command = Shlex::new(&command_string).collect();
	        } else if let Some(term) = std::env::var_os("TERMINAL") {
	            let term = match term.into_string() {
	            	Ok(s) => s,
	            	Err(_e) => return Err("invalid TERMINAL, couldn't convert".into()),
	            };
	            let mut command_new = vec![term, "-e".into()];
	            command_new.extend(command);
	            command = command_new;
	        } else {
	            return Err("couldn't find terminal".into()); // TODO: return correct error
	        };
	    }
	    if launch_cgroups {
	        let parsed = systemd_escape(&self.id);
	        let unit = format!(
	            "--unit=app-sirula-{}-{}",
	            parsed?,
	            id()
	        );
	        let mut command_new: Vec<String> = vec![
	            "systemd-run".into(),
	            "--scope".into(),
	            "--user".into(),
	            unit,
	        ];
	        command_new.extend(command);
	        command = command_new;
	    }
	
	    let mut exec = Command::new(&command[0]);
	    let mut exec = exec.args(&command[1..]);
	    if let Some(dir) = &self.path {
	        exec = exec.current_dir(dir)
	    }
	    if self.prefers_nondefault_gpu {
	        if let Some(prime) = gpu_variable {
	            exec = exec.env(prime, "1")
	        }
	    }
	
	    Ok(exec.spawn()?)
	}
}

pub fn systemd_escape(string: &String) -> Result<String, Box<dyn Error>> {
	let string = string.as_bytes();
	let mut out: Vec<u8> = Vec::with_capacity(string.len());
	let mut first = true;
	for s in string {
		let mut s: Vec<u8> = s.escape_ascii().collect();
		if first {
			if s[0] == b'.' {
				s = r"\x2e".as_bytes().into()
			}
			first = false;
		}
		if s[0] == b'-' {
			s = r"\x2d".as_bytes().into()
		} else if s[0] == b'/' {
			s[0] = b'-'
		}
		out.append(&mut s)
	}
	Ok(String::from_utf8(out)?)
}

fn get_exec_key(group: &Group) -> Option<String> {
    match get_key(group, "TryExec") {
        Some(try_exec) => match which(&try_exec) {
            Ok(_) => get_key(group, "Exec").or(Some(try_exec)),
            Err(_) => None,
        },
        None => get_key(group, "Exec"),
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
        Some(x) => match x.1.clone().into_values().next() {
            Some(x) => Some(x),
            None => Some(x.0.clone()),
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
