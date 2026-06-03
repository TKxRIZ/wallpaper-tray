use ksni::menu::StandardItem;
use ksni::{MenuItem, ToolTip, Tray};

use crate::{config, service, updater, wallpaper};

pub struct WallpaperTray {
    pub service:          String,
    pub recents:          Vec<wallpaper::Wallpaper>,
    pub active:           bool,
    pub update_available: Option<String>,
}

impl WallpaperTray {
    pub fn new() -> Self {
        let cfg     = config::load();
        let active  = service::is_active(&cfg.service_name);
        let recents = wallpaper::resolve_recents(&cfg.recent_wallpapers);
        Self { service: cfg.service_name, recents, active, update_available: None }
    }

    fn refresh(&mut self) {
        let cfg      = config::load();
        self.service = cfg.service_name.clone();
        self.active  = service::is_active(&cfg.service_name);
        self.recents = wallpaper::resolve_recents(&cfg.recent_wallpapers);
    }
}

impl Tray for WallpaperTray {
    fn id(&self) -> String { "wallpaper-tray".into() }

    fn icon_name(&self) -> String {
        if self.update_available.is_some() {
            "software-update-available".into()
        } else {
            "preferences-desktop-wallpaper".into()
        }
    }

    fn tool_tip(&self) -> ToolTip {
        let description = match (&self.update_available, self.active) {
            (Some(v), _) => format!("Update verfügbar: {v}"),
            (_, true)    => "Service aktiv".into(),
            _            => "Service inaktiv".into(),
        };
        ToolTip {
            title: "Wallpaper Engine – Linux".into(),
            description,
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let mut items: Vec<MenuItem<Self>> = Vec::new();

        // Update banner
        if let Some(ref version) = self.update_available {
            let v = version.clone();
            items.push(MenuItem::Standard(StandardItem {
                label: format!("↑  Update verfügbar — {v}"),
                activate: Box::new(|this: &mut Self| {
                    let cfg = config::load();
                    if let Err(e) = updater::download_and_apply(&cfg) {
                        eprintln!("Update fehlgeschlagen: {e}");
                    }
                }),
                ..Default::default()
            }));
            items.push(MenuItem::Separator);
        }

        // Recent wallpapers
        if !self.recents.is_empty() {
            for wp in &self.recents {
                let id    = wp.id.clone();
                let title = wp.title.clone();
                let svc   = self.service.clone();
                items.push(MenuItem::Standard(StandardItem {
                    label: title,
                    activate: Box::new(move |this: &mut Self| {
                        service::apply_wallpaper(&svc, &id);
                        this.refresh();
                    }),
                    ..Default::default()
                }));
            }
            items.push(MenuItem::Separator);
        }

        // Random wallpaper
        items.push(MenuItem::Standard(StandardItem {
            label: "Zufälliges Wallpaper".into(),
            enabled: !wallpaper::all_installed().is_empty(),
            activate: Box::new(|this: &mut Self| {
                let all = wallpaper::all_installed();
                if all.is_empty() { return; }
                let idx = (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos() as usize) % all.len();
                service::apply_wallpaper(&this.service, &all[idx].id);
                this.refresh();
            }),
            ..Default::default()
        }));

        items.push(MenuItem::Separator);

        // Status + manual pause/resume
        items.push(MenuItem::Standard(StandardItem {
            label:   if self.active { "● Aktiv".into() } else { "○ Inaktiv".into() },
            enabled: false,
            ..Default::default()
        }));

        if self.active {
            items.push(MenuItem::Standard(StandardItem {
                label: "⏸  Pausieren".into(),
                activate: Box::new(|this: &mut Self| {
                    service::stop(&this.service);
                    this.active = false;
                }),
                ..Default::default()
            }));
        } else {
            items.push(MenuItem::Standard(StandardItem {
                label: "▶  Fortsetzen".into(),
                activate: Box::new(|this: &mut Self| {
                    service::start(&this.service);
                    this.active = service::is_active(&this.service);
                }),
                ..Default::default()
            }));
        }

        items.push(MenuItem::Separator);

        items.push(MenuItem::Standard(StandardItem {
            label: "Öffnen".into(),
            activate: Box::new(|_| {
                let _ = std::process::Command::new("wallpaper-picker").spawn();
            }),
            ..Default::default()
        }));

        items.push(MenuItem::Standard(StandardItem {
            label: "Beenden".into(),
            activate: Box::new(|_| std::process::exit(0)),
            ..Default::default()
        }));

        items
    }
}
