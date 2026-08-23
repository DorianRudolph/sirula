use freedesktop_desktop_entry::{default_paths, get_languages_from_env, Group, Iter};
use log::{debug, error, info, warn};
use shlex::Shlex;
use which::which;

use std::{
    collections::HashMap,
    env::var,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
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
    pub name: String,
    pub exec: String,
}

macro_rules! skip_none {
    ($res:expr, $id:expr) => {
        match $res {
            Some(val) => val,
            None => {
                error!("skipping: {} (missing/wrong values)", &$id);
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
            let desktop_entry = skip_none!(entry.groups.0.get("Desktop Entry"), id);

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
                    debug!("skipping: {} (hidden)", &id);
                    continue;
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

                id,
                actions,
            };

            if let Some(app_entry) = out.insert(app_entry.id.clone(), app_entry) {
                // TODO: clone
                debug!("skipping: {} (overwritten)", &app_entry.id)
            }
        }
        out.into_values().collect()
    }
    pub fn run(
        &self,
        term_command: Option<&str>,
        launch_cgroups: (bool, bool),
        gpu_variable: Option<String>,
    ) {
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
            ("%i", &self.icon.clone().unwrap_or_default()), // icon TODO: clone!
            ("%c", &self.name),                             // name (translated)
            ("%k", ""),                                     // filename as uri > file > none
        ];
        let mut command_string = self.exec.clone();
        for replace_key in replace_keys {
            command_string = command_string.replace(replace_key.0, replace_key.1)
        }
        let mut command: Vec<String> = Shlex::new(&command_string).collect();

        if self.terminal {
            if let Some(term) = term_command {
                let command_string = term.to_string().replace("{}", &command_string);
                command = Shlex::new(&command_string).collect();
            } else if let Some(term) = std::env::var_os("TERMINAL") {
                let term = term.into_string().expect("couldn't convert to string");
                let mut command_new = vec![term, "-e".into()];
                command_new.extend(command);
                command = command_new;
            } else {
                return;
            };
        }
        if launch_cgroups.0 {
            let parsed = escape_name(&self.id);
            let unit = format!(
                "--unit=app-sirula-{}-{}",
                &parsed,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            );
            let mut command_new: Vec<String> = vec![
                "systemd-run".into(),
                "--slice=app.slice".into(),
                "--user".into(),
                unit,
            ];
            if !launch_cgroups.1 {
                command_new.push("--scope".into())
            }
            command_new.extend(command);
            command = command_new;
        }

        info!(
            "running \"{}\" with command \"{}\"",
            self.name,
            &command.join(" ")
        );

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

        exec.spawn().expect("Error launching app");
    }
}

pub fn setup_monitor(application: &gtk::Application) {
    use gio::prelude::FileExt;
    use glib::ObjectExt;
    use crate::clone;
    use gio::prelude::ActionGroupExt;
    use gio::prelude::FileMonitorExt;

    for (i, path) in default_paths().enumerate() {
        let dir = gio::File::for_path(&path);

        let monitor = dir
            .monitor_directory(
                gio::FileMonitorFlags::NONE,
                gio::Cancellable::NONE,
            )
            .expect("failed to monitor directory");

        monitor.connect_changed(clone!(application => move |_monitor, file, _other_file, event| {
            use gio::FileMonitorEvent::*;
            if event == Created || event == Changed || event== Deleted {
                application.activate_action("reload", None);
            };
            if let Some(name) = file.basename() {
                info!("file changed: {}/{} ({})", &path.display(), name.display(), event);
            }
        }));

        unsafe {
            application.set_data(&format!("file-monitor-{i}"), monitor);
        }
    }
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

// from systemd crate
pub fn escape_name(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len() * 2);
    for (index, b) in s.bytes().enumerate() {
        match b {
            b'/' => escaped.push('-'),
            // Do not escape '.' unless it's the first character
            b'.' if 0 < index => escaped.push(char::from(b)),
            // Do not escape _ and : and
            b'_' | b':' => escaped.push(char::from(b)),
            // all ASCII alphanumeric characters
            _ if b.is_ascii_alphanumeric() => escaped.push(char::from(b)),
            _ => escaped.push_str(&format!("\\x{b:02x}")),
        }
    }
    escaped
}
