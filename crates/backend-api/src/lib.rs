use std::env;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Hyprland,
    Wayland,
    X11,
    Desktop,
}

impl BackendKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hyprland => "hyprland",
            Self::Wayland => "wayland",
            Self::X11 => "x11",
            Self::Desktop => "desktop",
        }
    }

    pub fn parse(input: &str) -> Option<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "hyprland" => Some(Self::Hyprland),
            "wayland" => Some(Self::Wayland),
            "x11" => Some(Self::X11),
            "desktop" => Some(Self::Desktop),
            _ => None,
        }
    }
}

pub trait WallpaperBackend {
    fn kind(&self) -> BackendKind;
    fn display_name(&self) -> &'static str;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeEnvironment {
    pub desktop_session: Option<String>,
    pub current_desktop: Option<String>,
    pub wayland_display: Option<String>,
    pub display: Option<String>,
    pub hyprland_instance_signature: Option<String>,
}

impl RuntimeEnvironment {
    pub fn detect() -> Self {
        Self {
            desktop_session: env::var("DESKTOP_SESSION").ok(),
            current_desktop: env::var("XDG_CURRENT_DESKTOP").ok(),
            wayland_display: env::var("WAYLAND_DISPLAY").ok(),
            display: env::var("DISPLAY").ok(),
            hyprland_instance_signature: env::var("HYPRLAND_INSTANCE_SIGNATURE").ok(),
        }
    }

    pub fn is_hyprland(&self) -> bool {
        self.hyprland_instance_signature.is_some()
            || self
                .desktop_session
                .as_deref()
                .map(|value| value.eq_ignore_ascii_case("hyprland"))
                .unwrap_or(false)
            || self
                .current_desktop
                .as_deref()
                .map(|value| value.eq_ignore_ascii_case("hyprland"))
                .unwrap_or(false)
    }

    pub fn is_wayland(&self) -> bool {
        self.wayland_display.is_some()
    }

    pub fn is_x11(&self) -> bool {
        self.display.is_some()
    }

    pub fn is_live_session_ready(&self, backend: BackendKind) -> bool {
        match backend {
            BackendKind::Hyprland => {
                self.wayland_display.is_some() && self.hyprland_instance_signature.is_some()
            }
            BackendKind::Wayland | BackendKind::Desktop => self.wayland_display.is_some(),
            BackendKind::X11 => self.display.is_some(),
        }
    }

    pub fn detect_backend(&self, preferred_backend: Option<&str>) -> BackendKind {
        if let Some(preferred) = preferred_backend.and_then(BackendKind::parse) {
            let preferred_is_available = match preferred {
                BackendKind::Hyprland => self.is_hyprland(),
                BackendKind::Wayland | BackendKind::Desktop => self.is_wayland(),
                BackendKind::X11 => self.is_x11(),
            };
            if preferred_is_available {
                return preferred;
            }
        }

        if self.is_hyprland() {
            BackendKind::Hyprland
        } else if self.is_wayland() {
            BackendKind::Wayland
        } else if self.is_x11() {
            BackendKind::X11
        } else {
            BackendKind::Desktop
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallpaperKind {
    Image,
    Video,
}

impl WallpaperKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalingMode {
    Fill,
    Fit,
    Center,
}

impl ScalingMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fill => "fill",
            Self::Fit => "fit",
            Self::Center => "center",
        }
    }

    pub fn parse(input: &str) -> Option<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "fill" => Some(Self::Fill),
            "fit" => Some(Self::Fit),
            "center" => Some(Self::Center),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionMode {
    None,
    Fade,
}

impl TransitionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Fade => "fade",
        }
    }

    pub fn parse(input: &str) -> Option<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "none" => Some(Self::None),
            "fade" => Some(Self::Fade),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMode {
    Infinite,
    Once,
}

impl LoopMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Infinite => "infinite",
            Self::Once => "once",
        }
    }

    pub fn parse(input: &str) -> Option<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "infinite" => Some(Self::Infinite),
            "once" => Some(Self::Once),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WallpaperSpec {
    pub output: String,
    pub kind: WallpaperKind,
    pub path: String,
    pub scaling: ScalingMode,
    pub transition: TransitionMode,
    pub muted: bool,
    pub loop_mode: LoopMode,
}

impl WallpaperSpec {
    pub fn validate(&self) -> Result<(), String> {
        if self.output.trim().is_empty() {
            return Err("output name cannot be empty".to_string());
        }

        if self.path.trim().is_empty() {
            return Err("wallpaper path cannot be empty".to_string());
        }

        if !Path::new(&self.path).exists() {
            return Err(format!("wallpaper path does not exist: {}", self.path));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{BackendKind, RuntimeEnvironment};

    #[test]
    fn detects_hyprland_over_generic_wayland() {
        let env = RuntimeEnvironment {
            desktop_session: Some("hyprland".to_string()),
            current_desktop: Some("Hyprland".to_string()),
            wayland_display: Some("wayland-1".to_string()),
            display: None,
            hyprland_instance_signature: Some("abc".to_string()),
        };

        assert_eq!(env.detect_backend(None), BackendKind::Hyprland);
        assert!(env.is_live_session_ready(BackendKind::Hyprland));
    }

    #[test]
    fn falls_back_to_detected_backend_when_preference_is_unavailable() {
        let env = RuntimeEnvironment {
            desktop_session: None,
            current_desktop: None,
            wayland_display: None,
            display: Some(":0".to_string()),
            hyprland_instance_signature: None,
        };

        assert_eq!(env.detect_backend(Some("hyprland")), BackendKind::X11);
        assert!(env.is_live_session_ready(BackendKind::X11));
    }
}
