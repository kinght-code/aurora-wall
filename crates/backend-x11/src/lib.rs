use aurora_wall_backend_api::{BackendKind, WallpaperBackend};

#[derive(Debug, Default)]
pub struct X11Backend;

impl WallpaperBackend for X11Backend {
    fn kind(&self) -> BackendKind {
        BackendKind::X11
    }

    fn display_name(&self) -> &'static str {
        "X11 backend"
    }
}
