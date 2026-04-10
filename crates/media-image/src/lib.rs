#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageMode {
    Fill,
    Fit,
    Center,
}

impl ImageMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fill => "fill",
            Self::Fit => "fit",
            Self::Center => "center",
        }
    }
}

