use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::Config;

const TRAY_VERSION:   &str = env!("CARGO_PKG_VERSION");
const TRAY_ASSET:     &str = "wallpaper-tray-linux-x86_64";
const PICKER_VERSION: &str = env!("CARGO_PKG_VERSION"); // placeholder — picker reads from its own __init__.py

#[derive(Deserialize)]
struct VersionResponse {
    version: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ComponentState {
    pub current:    String,
    pub latest:     String,
    pub has_update: bool,
}

#[derive(Serialize, Deserialize, Default)]
pub struct UpdateState {
    pub checked_at: u64,
    pub tray:       ComponentState,
    pub picker:     ComponentState,
}

fn state_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".cache/wallpaper-picker/update-state.json")
}

pub fn check_update(cfg: &Config) -> Option<String> {
    if cfg.update_url.is_empty() {
        return None;
    }
    fetch_latest(cfg, "tray").and_then(|latest| {
        if parse_version(&latest) > parse_version(TRAY_VERSION) {
            Some(latest)
        } else {
            None
        }
    })
}

/// Check both tray + picker updates and write the state file.
/// Called once on startup in a background thread.
pub fn check_and_write_state(cfg: &Config) {
    if cfg.update_url.is_empty() {
        return;
    }

    let tray_latest   = fetch_latest(cfg, "tray").unwrap_or_default();
    let picker_latest = fetch_latest(cfg, "picker").unwrap_or_default();
    let picker_current = read_picker_version(cfg);

    let state = UpdateState {
        checked_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        tray: ComponentState {
            current:    TRAY_VERSION.to_string(),
            latest:     tray_latest.clone(),
            has_update: !tray_latest.is_empty()
                && parse_version(&tray_latest) > parse_version(TRAY_VERSION),
        },
        picker: ComponentState {
            current:    picker_current.clone(),
            latest:     picker_latest.clone(),
            has_update: !picker_latest.is_empty()
                && !picker_current.is_empty()
                && parse_version(&picker_latest) > parse_version(&picker_current),
        },
    };

    if let Ok(json) = serde_json::to_string_pretty(&state) {
        let path = state_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, json);
    }
}

fn fetch_latest(cfg: &Config, app: &str) -> Option<String> {
    let mut url = format!(
        "{}/v1/version?channel={}&app={}",
        cfg.update_url, cfg.update_channel, app
    );
    if !cfg.update_key.is_empty() {
        url.push_str(&format!("&key={}", cfg.update_key));
    }

    let resp: VersionResponse = ureq::get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .ok()?
        .into_json()
        .ok()?;

    Some(resp.version)
}

fn read_picker_version(cfg: &Config) -> String {
    // Read __version__ from wallpaper_picker/__init__.py
    // Project dir is detected relative to the wallpaper-picker config location
    let home = std::env::var("HOME").unwrap_or_default();
    let init = PathBuf::from(&home)
        .join("wallpaper-picker/wallpaper_picker/__init__.py");
    let Ok(content) = fs::read_to_string(&init) else {
        return String::new();
    };
    // Parse: __version__ = "1.0.3"
    for line in content.lines() {
        if line.starts_with("__version__") {
            if let Some(v) = line.split('"').nth(1) {
                return v.to_string();
            }
            if let Some(v) = line.split('\'').nth(1) {
                return v.to_string();
            }
        }
    }
    String::new()
}

pub fn download_and_apply(cfg: &Config) -> Result<(), String> {
    let mut url = format!(
        "{}/v1/download?channel={}&app=tray&asset={}",
        cfg.update_url, cfg.update_channel, TRAY_ASSET
    );
    if !cfg.update_key.is_empty() {
        url.push_str(&format!("&key={}", cfg.update_key));
    }

    let tmp = std::env::temp_dir().join("wallpaper-tray-update");
    let resp = ureq::get(&url)
        .timeout(std::time::Duration::from_secs(120))
        .call()
        .map_err(|e| format!("Download fehlgeschlagen: {e}"))?;

    let mut reader = resp.into_reader();
    let mut file = fs::File::create(&tmp)
        .map_err(|e| format!("Temp-Datei: {e}"))?;
    std::io::copy(&mut reader, &mut file)
        .map_err(|e| format!("Schreiben fehlgeschlagen: {e}"))?;
    drop(file);

    fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))
        .map_err(|e| format!("chmod: {e}"))?;

    let target = std::env::current_exe()
        .map_err(|e| format!("Binary-Pfad: {e}"))?;
    fs::rename(&tmp, &target)
        .map_err(|e| format!("Ersetzen fehlgeschlagen: {e}"))?;

    restart(&target)
}

fn restart(binary: &PathBuf) -> Result<(), String> {
    use std::os::unix::process::CommandExt;
    let args: Vec<String> = std::env::args().collect();
    let err = std::process::Command::new(binary).args(&args[1..]).exec();
    Err(format!("exec fehlgeschlagen: {err}"))
}

fn parse_version(v: &str) -> (u64, u64, u64) {
    let v = v.trim_start_matches('v');
    let mut parts = v.split('.').filter_map(|p| p.parse::<u64>().ok());
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}
