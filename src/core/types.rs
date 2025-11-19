/// Core typed IDs and handles to avoid mixing up card database code with internal handle index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CardId(pub u32);

impl CardId {
    pub fn new(idx: u32) -> Self {
        CardId(idx)
    }
    pub fn as_u32(self) -> u32 { self.0 }
}

impl Default for CardId {
    fn default() -> Self { CardId(0) }
}
