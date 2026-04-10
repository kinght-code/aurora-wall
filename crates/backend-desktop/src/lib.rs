use aurora_wall_backend_api::{BackendKind, WallpaperBackend};

#[derive(Debug, Default)]
pub struct DesktopBackend;

impl WallpaperBackend for DesktopBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Desktop
    }

    fn display_name(&self) -> &'static str {
        "Desktop integration backend"
    }
}
