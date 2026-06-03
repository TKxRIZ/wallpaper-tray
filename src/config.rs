use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_service")]
    pub service_name: String,
    #[serde(default)]
    pub recent_wallpapers: Vec<String>,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_container")]
    pub container: String,
    #[serde(default)]
    pub assets_dir: String,
    #[serde(default = "default_fps")]
    pub fps: u32,
    #[serde(default = "default_update_url")]
    pub update_url: String,
    #[serde(default = "default_update_channel")]
    pub update_channel: String,
    #[serde(default)]
    pub update_key: String,
    #[serde(default = "default_fullscreen_pause")]
    pub fullscreen_pause: bool,
    #[serde(default)]
    pub project_dir: String,
}


impl Default for Config {
    fn default() -> Self {
        Self {
            service_name: default_service(),
            recent_wallpapers: vec![],
            mode: default_mode(),
            container: default_container(),
            assets_dir: String::new(),
            fps: default_fps(),
            update_url: default_update_url(),
            update_channel: default_update_channel(),
            update_key: String::new(),
            fullscreen_pause: default_fullscreen_pause(),
            project_dir: String::new(),
        }
    }
}

fn default_service()        -> String { "linux-wallpaperengine".into() }
fn default_mode()           -> String { "distrobox".into() }
fn default_container()      -> String { "wallpaperengine".into() }
fn default_fps()            -> u32    { 30 }
fn default_update_url()       -> String { "https://wallpaper.dev.tkxriz.me".into() }
fn default_update_channel()   -> String { "stable".into() }
fn default_fullscreen_pause() -> bool   { true }

pub fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default())
}

pub fn load() -> Config {
    let path = home_dir().join(".config/wallpaper-picker/config.json");
    let Ok(data) = fs::read_to_string(&path) else {
        return Config::default();
    };
    serde_json::from_str(&data).unwrap_or_default()
}
