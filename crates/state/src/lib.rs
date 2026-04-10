use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestorePolicy {
    Disabled,
    LastKnownGood,
}

impl Default for RestorePolicy {
    fn default() -> Self {
        Self::LastKnownGood
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedState {
    pub last_applied_backend: String,
    pub applied_items: usize,
}

impl AppliedState {
    pub fn new(last_applied_backend: impl Into<String>, applied_items: usize) -> Self {
        Self {
            last_applied_backend: last_applied_backend.into(),
            applied_items,
        }
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let raw = fs::read_to_string(path)?;
        let mut backend = String::new();
        let mut applied_items = 0usize;

        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = trimmed.split_once('=') {
                match key.trim() {
                    "last_applied_backend" => backend = value.trim().to_string(),
                    "applied_items" => {
                        applied_items = value.trim().parse().unwrap_or(0);
                    }
                    _ => {}
                }
            }
        }

        Ok(Self {
            last_applied_backend: backend,
            applied_items,
        })
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let body = format!(
            "last_applied_backend={}\napplied_items={}\n",
            self.last_applied_backend, self.applied_items
        );

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
