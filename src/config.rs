use serde::{Deserialize, Serialize};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use crate::systemd::is_valid_service_name;

const MAX_CONFIG_BYTES: u64 = 64 * 1024;
const MAX_LABEL_CHARS: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnedService {
    pub id: String,
    pub label: String,
}

pub fn get_config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("io", "github.yemresolenci", "CosmicExtAppletSystemdManager")
        .map(|proj_dirs| proj_dirs.config_dir().join("config.json"))
}

fn get_legacy_config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("io", "github.yemresolenci", "CosmicSystemdManager")
        .map(|proj_dirs| proj_dirs.config_dir().join("config.json"))
}

pub fn load_pinned_services() -> Vec<PinnedService> {
    let target_path = get_config_path();
    let legacy_path = get_legacy_config_path();

    let path_to_read = match (&target_path, &legacy_path) {
        (Some(tp), _) if tp.exists() => Some(tp),
        (_, Some(lp)) if lp.exists() => Some(lp),
        (Some(tp), _) => Some(tp),
        _ => None,
    };

    if let Some(path) = path_to_read {
        let Ok(meta) = fs::metadata(path) else {
            return Vec::new();
        };
        if !meta.is_file() || meta.len() > MAX_CONFIG_BYTES {
            return Vec::new();
        }
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(services) = serde_json::from_str::<Vec<PinnedService>>(&content) {
                return services
                    .into_iter()
                    .filter(|s| is_valid_service_name(&s.id))
                    .map(|mut s| {
                        s.label = s.label.chars().take(MAX_LABEL_CHARS).collect();
                        s
                    })
                    .collect();
            }
        }
    }
    Vec::new()
}

pub fn save_pinned_services(services: &[PinnedService]) {
    if let Some(path) = get_config_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(services) {
            if fs::write(&path, content).is_ok() {
                #[cfg(unix)]
                {
                    let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
                }
            }
        }
    }
}
