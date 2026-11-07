use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use zbus::{proxy, Connection, Result};

/// Maximum length of a valid systemd unit name (`UNIT_NAME_MAX`).
const MAX_UNIT_NAME_LEN: usize = 256;

/// Validates that `name` is a safe, well-formed `.service` unit identifier.
/// Rejects path traversal, control characters, template definitions (`@.service`),
/// and non-service unit types (such as `.target`, `.socket`, or `.mount`).
pub fn is_valid_service_name(name: &str) -> bool {
    if name.is_empty() || name.len() > MAX_UNIT_NAME_LEN {
        return false;
    }
    if !name.ends_with(".service") || name.ends_with("@.service") {
        return false;
    }
    if name.contains('/') || name.contains('\0') || name.contains("..") {
        return false;
    }
    name.chars().all(|c| {
        c.is_ascii_alphanumeric() || matches!(c, ':' | '_' | '-' | '.' | '\\' | '@')
    })
}

/// Represents a single unit returned by the systemd ListUnits D-Bus call.
/// The structure signature maps to `(ssssssouso)`.
#[derive(Debug, Clone, Serialize, Deserialize, zbus::zvariant::Type)]
pub struct UnitInfo {
    pub id: String,
    pub description: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub following: String,
    pub unit_path: zbus::zvariant::OwnedObjectPath,
    pub job_id: u32,
    pub job_type: String,
    pub job_path: zbus::zvariant::OwnedObjectPath,
}

#[derive(Debug, Clone)]
struct CachedUnitFile {
    id: String,
    file_state: String,
    description: String,
}

#[proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1"
)]
pub trait SystemdManager {
    /// Returns a list of all currently loaded units.
    fn list_units(&self) -> Result<Vec<UnitInfo>>;

    /// Returns a list of all unit files on disk: `(file_path, unit_file_state)`.
    fn list_unit_files(&self) -> Result<Vec<(String, String)>>;

    /// Start a unit by name (automatically loads it if unloaded).
    fn start_unit(&self, name: &str, mode: &str) -> Result<zbus::zvariant::OwnedObjectPath>;

    /// Stop a unit by name.
    fn stop_unit(&self, name: &str, mode: &str) -> Result<zbus::zvariant::OwnedObjectPath>;

    /// Restart a unit by name.
    fn restart_unit(&self, name: &str, mode: &str) -> Result<zbus::zvariant::OwnedObjectPath>;

    /// Load a unit by name and return its D-Bus object path.
    fn load_unit(&self, name: &str) -> Result<zbus::zvariant::OwnedObjectPath>;
}

#[proxy(
    interface = "org.freedesktop.systemd1.Unit",
    default_service = "org.freedesktop.systemd1"
)]
pub trait SystemdUnit {
    #[zbus(property)]
    fn active_state(&self) -> Result<String>;

    #[zbus(property)]
    fn sub_state(&self) -> Result<String>;

    #[zbus(property)]
    fn load_state(&self) -> Result<String>;

    #[zbus(property)]
    fn description(&self) -> Result<String>;
}

/// Safely reads the `Description=` field from a systemd unit file on disk.
/// Verifies the path is a regular file of bounded size and reads at most 8 KiB.
fn read_unit_file_description(file_path: &str) -> String {
    const ALLOWED_PREFIXES: &[&str] = &[
        "/etc/systemd/",
        "/run/systemd/",
        "/usr/lib/systemd/",
        "/lib/systemd/",
        "/usr/local/lib/systemd/",
    ];
    if !ALLOWED_PREFIXES.iter().any(|p| file_path.starts_with(p)) {
        return String::new();
    }

    let Ok(meta) = fs::metadata(file_path) else {
        return String::new();
    };
    if !meta.is_file() || meta.len() > 64 * 1024 {
        return String::new();
    }

    let Ok(file) = File::open(file_path) else {
        return String::new();
    };
    let reader = BufReader::new(file.take(8 * 1024));
    for line in reader.lines().map_while(std::result::Result::ok) {
        let trimmed = line.trim();
        if let Some(desc) = trimmed.strip_prefix("Description=") {
            return desc.trim().to_string();
        }
    }
    String::new()
}

/// A client wrapper to simplify interacting with the systemd D-Bus API.
#[derive(Clone, Debug)]
pub struct SystemdClient {
    connection: Connection,
    unit_files_cache: Arc<Mutex<Vec<CachedUnitFile>>>,
}

impl SystemdClient {
    /// Initialize a new D-Bus system connection.
    pub async fn new() -> Result<Self> {
        let connection = Connection::system().await?;
        let client = Self {
            connection,
            unit_files_cache: Arc::new(Mutex::new(Vec::new())),
        };
        let _ = client.refresh_unit_files().await;
        Ok(client)
    }

    /// Refresh the cached list of `.service` unit files from systemd (`ListUnitFiles`).
    pub async fn refresh_unit_files(&self) -> Result<()> {
        let proxy = SystemdManagerProxy::new(&self.connection).await?;
        let unit_files = proxy.list_unit_files().await?;

        let mut cached = Vec::new();
        for (file_path, file_state) in unit_files {
            if file_state == "masked" || file_state == "masked-runtime" {
                continue;
            }
            let Some(file_name) = Path::new(&file_path).file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !is_valid_service_name(file_name) {
                continue;
            }
            let description = read_unit_file_description(&file_path);
            cached.push(CachedUnitFile {
                id: file_name.to_string(),
                file_state,
                description,
            });
        }

        if let Ok(mut guard) = self.unit_files_cache.lock() {
            *guard = cached;
        }
        Ok(())
    }

    /// Fetch all `.service` units, combining loaded units (`ListUnits`) with
    /// unloaded/disabled unit files on disk (`ListUnitFiles`).
    pub async fn list_services(&self) -> Result<Vec<UnitInfo>> {
        let need_cache_init = self
            .unit_files_cache
            .lock()
            .map(|g| g.is_empty())
            .unwrap_or(true);
        if need_cache_init {
            let _ = self.refresh_unit_files().await;
        }

        let proxy = SystemdManagerProxy::new(&self.connection).await?;
        let units = proxy.list_units().await?;

        let mut service_map: HashMap<String, UnitInfo> = units
            .into_iter()
            .filter(|u| is_valid_service_name(&u.id) && u.load_state != "not-found")
            .map(|u| (u.id.clone(), u))
            .collect();

        let default_path =
            zbus::zvariant::OwnedObjectPath::try_from("/org/freedesktop/systemd1")?;

        if let Ok(cached_files) = self.unit_files_cache.lock() {
            for unit_file in cached_files.iter() {
                service_map
                    .entry(unit_file.id.clone())
                    .or_insert_with(|| UnitInfo {
                        id: unit_file.id.clone(),
                        description: unit_file.description.clone(),
                        load_state: unit_file.file_state.clone(),
                        active_state: "inactive".to_string(),
                        sub_state: "dead".to_string(),
                        following: String::new(),
                        unit_path: default_path.clone(),
                        job_id: 0,
                        job_type: String::new(),
                        job_path: default_path.clone(),
                    });
            }
        }

        let mut services: Vec<UnitInfo> = service_map.into_values().collect();
        services.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(services)
    }

    /// Fetch all `.service` units after refreshing the unit files cache from disk.
    pub async fn list_services_full(&self) -> Result<Vec<UnitInfo>> {
        let _ = self.refresh_unit_files().await;
        self.list_services().await
    }

    /// Query the current `(active_state, sub_state)` of a unit directly via D-Bus.
    pub async fn get_unit_state(&self, unit_id: &str) -> Result<(String, String)> {
        if !is_valid_service_name(unit_id) {
            return Err(zbus::Error::Failure("Invalid service unit name".to_string()));
        }
        let manager = SystemdManagerProxy::new(&self.connection).await?;
        let unit_path = manager.load_unit(unit_id).await?;
        let unit_proxy = SystemdUnitProxy::builder(&self.connection)
            .path(unit_path)?
            .build()
            .await?;
        let active_state = unit_proxy.active_state().await?;
        let sub_state = unit_proxy.sub_state().await?;
        Ok((active_state, sub_state))
    }

    /// Wait briefly for an in-flight start/stop/restart transition to settle.
    async fn wait_for_transition(&self, unit_id: &str, target_active: bool) {
        for _ in 0..20 {
            tokio::time::sleep(Duration::from_millis(150)).await;
            if let Ok((active_state, sub_state)) = self.get_unit_state(unit_id).await {
                if target_active {
                    // Starting or restarting: wait while actively starting/deactivating/reloading
                    if active_state == "active" {
                        // Brief extra check so fast-failing simple services that enter auto-restart are caught
                        tokio::time::sleep(Duration::from_millis(150)).await;
                        break;
                    }
                    if active_state == "failed"
                        || active_state == "inactive"
                        || sub_state == "auto-restart"
                    {
                        break;
                    }
                } else {
                    // Stopping: wait until inactive or failed
                    if active_state == "inactive" || active_state == "failed" {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    pub async fn start_unit(&self, unit_id: &str) -> Result<()> {
        if !is_valid_service_name(unit_id) {
            return Err(zbus::Error::Failure("Invalid service unit name".to_string()));
        }
        let proxy = SystemdManagerProxy::new(&self.connection).await?;
        proxy.start_unit(unit_id, "replace").await?;
        self.wait_for_transition(unit_id, true).await;
        Ok(())
    }

    pub async fn stop_unit(&self, unit_id: &str) -> Result<()> {
        if !is_valid_service_name(unit_id) {
            return Err(zbus::Error::Failure("Invalid service unit name".to_string()));
        }
        let proxy = SystemdManagerProxy::new(&self.connection).await?;
        proxy.stop_unit(unit_id, "replace").await?;
        self.wait_for_transition(unit_id, false).await;
        Ok(())
    }

    pub async fn restart_unit(&self, unit_id: &str) -> Result<()> {
        if !is_valid_service_name(unit_id) {
            return Err(zbus::Error::Failure("Invalid service unit name".to_string()));
        }
        let proxy = SystemdManagerProxy::new(&self.connection).await?;
        proxy.restart_unit(unit_id, "replace").await?;
        self.wait_for_transition(unit_id, true).await;
        Ok(())
    }
}
