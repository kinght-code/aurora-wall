use aurora_wall_backend_api::{BackendKind, WallpaperBackend};
use aurora_wall_media_image::ImageMode;

#[derive(Debug, Default)]
pub struct WaylandBackend;

impl WallpaperBackend for WaylandBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Wayland
    }

    fn display_name(&self) -> &'static str {
        "Generic Wayland backend"
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputAssignment {
    pub output_name: String,
    pub image_mode: ImageMode,
}
