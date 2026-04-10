use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketPath(PathBuf);

impl Default for SocketPath {
    fn default() -> Self {
        let path = env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("aurora-wall.sock");

        Self(path)
    }
}

impl SocketPath {
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}
