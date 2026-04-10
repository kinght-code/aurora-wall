use aurora_wall_backend_api::{BackendKind, WallpaperBackend};
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
