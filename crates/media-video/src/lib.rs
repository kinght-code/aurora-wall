#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopPolicy {
    Infinite,
    Once,
}

impl LoopPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Infinite => "infinite",
            Self::Once => "once",
        }
    }
}
