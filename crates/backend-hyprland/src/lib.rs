use aurora_wall_backend_api::{BackendKind, WallpaperBackend};
use std::env;
use std::io;
use std::process::Command;

#[derive(Debug, Default)]
pub struct HyprlandBackend;

impl WallpaperBackend for HyprlandBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Hyprland
    }

    fn display_name(&self) -> &'static str {
        "Hyprland backend"
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HyprlandEnvironment {
    pub desktop_session: Option<String>,
    pub current_desktop: Option<String>,
    pub wayland_display: Option<String>,
    pub hyprland_instance_signature: Option<String>,
}

impl HyprlandEnvironment {
    pub fn detect() -> Self {
        Self {
            desktop_session: env::var("DESKTOP_SESSION").ok(),
            current_desktop: env::var("XDG_CURRENT_DESKTOP").ok(),
            wayland_display: env::var("WAYLAND_DISPLAY").ok(),
            hyprland_instance_signature: env::var("HYPRLAND_INSTANCE_SIGNATURE").ok(),
        }
    }

    pub fn is_hyprland(&self) -> bool {
        self.desktop_session
            .as_deref()
            .map(|value| value.eq_ignore_ascii_case("hyprland"))
            .unwrap_or(false)
            || self
                .current_desktop
                .as_deref()
                .map(|value| value.eq_ignore_ascii_case("hyprland"))
                .unwrap_or(false)
    }

    pub fn is_live_session_ready(&self) -> bool {
        self.wayland_display.is_some() && self.hyprland_instance_signature.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HyprlandMonitor {
    pub name: String,
    pub description: String,
}

pub fn list_monitors() -> io::Result<Vec<HyprlandMonitor>> {
    let output = Command::new("hyprctl").arg("monitors").output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::other(stderr.trim().to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut monitors = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_desc: Option<String> = None;

    for line in stdout.lines() {
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("Monitor ") {
            if let Some(name) = current_name.take() {
                monitors.push(HyprlandMonitor {
                    name,
                    description: current_desc.take().unwrap_or_default(),
                });
            }

            let name = rest
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_string();
            current_name = Some(name);
            current_desc = None;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("description:") {
            current_desc = Some(rest.trim().to_string());
        }
    }

    if let Some(name) = current_name.take() {
        monitors.push(HyprlandMonitor {
            name,
            description: current_desc.take().unwrap_or_default(),
        });
    }

    Ok(monitors)
}
