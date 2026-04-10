use aurora_wall_backend_api::{
    LoopMode, ScalingMode, TransitionMode, WallpaperKind, WallpaperSpec,
};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RestorePolicy {
    Disabled,
    #[default]
    LastKnownGood,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedState {
    pub last_applied_backend: String,
    pub wallpapers: Vec<WallpaperSpec>,
}

impl AppliedState {
    pub fn new(last_applied_backend: impl Into<String>, wallpapers: Vec<WallpaperSpec>) -> Self {
        Self {
            last_applied_backend: last_applied_backend.into(),
            wallpapers,
        }
    }

    pub fn applied_items(&self) -> usize {
        self.wallpapers.len()
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let raw = fs::read_to_string(path)?;
        let mut backend = String::new();
        let mut wallpapers = Vec::new();

        for (index, line) in raw.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = trimmed.split_once('=') {
                match key.trim() {
                    "last_applied_backend" => backend = value.trim().to_string(),
                    "wallpaper" => {
                        wallpapers.push(parse_wallpaper(value.trim()).map_err(|msg| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                format!("invalid wallpaper entry on line {}: {}", index + 1, msg),
                            )
                        })?)
                    }
                    _ => {}
                }
            }
        }

        Ok(Self {
            last_applied_backend: backend,
            wallpapers,
        })
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut body = format!("last_applied_backend={}\n", self.last_applied_backend);
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

pub fn default_state_path() -> PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".local/state"))
        })
        .unwrap_or_else(|| PathBuf::from("."));

    base.join("aurora-wall").join("state.conf")
}

impl RestorePolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::LastKnownGood => "last-known-good",
        }
    }
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
    let scaling =
        ScalingMode::parse(fields[3]).ok_or_else(|| "invalid scaling mode".to_string())?;
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

#[cfg(test)]
mod tests {
    use super::AppliedState;
    use aurora_wall_backend_api::{
        LoopMode, ScalingMode, TransitionMode, WallpaperKind, WallpaperSpec,
    };

    #[test]
    fn state_round_trips_wallpaper_snapshot() {
        let dir = std::env::temp_dir().join(format!("aurora-wall-state-{}", std::process::id()));
        let path = dir.join("state.conf");
        let state = AppliedState::new(
            "wayland",
            vec![
                WallpaperSpec {
                    output: "eDP-1".to_string(),
                    kind: WallpaperKind::Image,
                    path: "/tmp/still.jpg".to_string(),
                    scaling: ScalingMode::Fill,
                    transition: TransitionMode::Fade,
                    muted: true,
                    loop_mode: LoopMode::Infinite,
                },
                WallpaperSpec {
                    output: "HDMI-1".to_string(),
                    kind: WallpaperKind::Video,
                    path: "/tmp/live.mp4".to_string(),
                    scaling: ScalingMode::Fill,
                    transition: TransitionMode::None,
                    muted: false,
                    loop_mode: LoopMode::Once,
                },
            ],
        );

        state.save(&path).expect("state should save");
        let loaded = AppliedState::load(&path).expect("state should load");

        assert_eq!(loaded.last_applied_backend, "wayland");
        assert_eq!(loaded.applied_items(), 2);
        assert_eq!(loaded.wallpapers, state.wallpapers);

        std::fs::remove_file(&path).ok();
        std::fs::remove_dir_all(&dir).ok();
    }
}
