use aurora_wall_backend_api::{LoopMode, ScalingMode, TransitionMode, WallpaperKind, WallpaperSpec};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub target_family: &'static str,
    pub preferred_backend: &'static str,
    pub restore_on_login: bool,
    pub library_dir: PathBuf,
    pub wallpapers: Vec<WallpaperSpec>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            target_family: "cross-desktop-linux",
            preferred_backend: "hyprland",
            restore_on_login: true,
            library_dir: default_library_dir(),
            wallpapers: Vec::new(),
        }
    }
}

impl AppConfig {
    pub fn load(path: &Path) -> io::Result<Self> {
        let raw = fs::read_to_string(path)?;
        parse_config(&raw)
    }

    pub fn load_or_default(path: &Path) -> io::Result<Self> {
        match Self::load(path) {
            Ok(config) => Ok(config),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error),
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut body = String::new();
        body.push_str("# aurora-wall config\n");
        body.push_str("backend=hyprland\n");
        body.push_str(&format!("restore_on_login={}\n", self.restore_on_login));
        body.push_str(&format!("library_dir={}\n", self.library_dir.display()));

        for wallpaper in &self.wallpapers {
            body.push_str(&format!(
                "wallpaper={}|{}|{}|{}|{}|{}|{}\n",
                wallpaper.output,
                wallpaper.kind.as_str(),
                wallpaper.path,
                wallpaper.scaling.as_str(),
                wallpaper.transition.as_str(),
                if wallpaper.muted { "muted" } else { "unmuted" },
                wallpaper.loop_mode.as_str()
            ));
        }

        fs::write(path, body)
    }
}

pub fn default_config_path() -> PathBuf {
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));

    base.join("aurora-wall").join("config.conf")
}

pub fn default_library_dir() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local/share/aurora-wall/wallpapers")
}

fn parse_config(raw: &str) -> io::Result<AppConfig> {
    let mut config = AppConfig::default();
    config.wallpapers.clear();

    for (index, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let (key, value) = trimmed.split_once('=').ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid config line {}: {}", index + 1, trimmed),
            )
        })?;

        match key.trim() {
            "backend" => {
                if value.trim() != "hyprland" {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "only backend=hyprland is supported in version 1",
                    ));
                }
            }
            "restore_on_login" => {
                config.restore_on_login = parse_bool(value.trim()).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("invalid boolean value: {}", value.trim()),
                    )
                })?;
            }
            "library_dir" => config.library_dir = PathBuf::from(value.trim()),
            "wallpaper" => config.wallpapers.push(parse_wallpaper(value.trim()).map_err(|msg| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid wallpaper entry on line {}: {}", index + 1, msg),
                )
            })?),
            _ => {}
        }
    }

    Ok(config)
}

fn parse_wallpaper(raw: &str) -> Result<WallpaperSpec, String> {
    let fields: Vec<&str> = raw.split('|').collect();
    if fields.len() != 7 {
        return Err("wallpaper requires 7 pipe-separated fields".to_string());
    }

    let kind = match fields[1].trim().to_ascii_lowercase().as_str() {
        "image" => WallpaperKind::Image,
        "video" => WallpaperKind::Video,
        other => return Err(format!("unsupported wallpaper kind: {}", other)),
    };

    let scaling = ScalingMode::parse(fields[3]).ok_or_else(|| "invalid scaling mode".to_string())?;
    let transition =
        TransitionMode::parse(fields[4]).ok_or_else(|| "invalid transition mode".to_string())?;
    let muted = match fields[5].trim().to_ascii_lowercase().as_str() {
        "muted" => true,
        "unmuted" => false,
        _ => return Err("mute field must be muted or unmuted".to_string()),
    };
    let loop_mode = LoopMode::parse(fields[6]).ok_or_else(|| "invalid loop mode".to_string())?;

    Ok(WallpaperSpec {
        output: fields[0].trim().to_string(),
        kind,
        path: fields[2].trim().to_string(),
        scaling,
        transition,
        muted,
        loop_mode,
    })
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "yes" | "1" => Some(true),
        "false" | "no" | "0" => Some(false),
        _ => None,
    }
}
