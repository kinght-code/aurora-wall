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
}

pub trait WallpaperBackend {
    fn kind(&self) -> BackendKind;
    fn display_name(&self) -> &'static str;
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
