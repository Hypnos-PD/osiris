use crate::core::types::CardId;

/// Basic Effect structure for now
pub struct Effect {
    pub id: u32,
    pub owner: CardId,
    pub description: u32,
    pub code: u32,
    pub flag: u32,
}

impl Effect {
    pub fn new(id: u32, owner: CardId, description: u32, code: u32, flag: u32) -> Self {
        Effect { id, owner, description, code, flag }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::CardId;

    #[test]
    fn effect_constructs() {
        let e = Effect::new(1, CardId::new(0), 10, 100, 0);
        assert_eq!(e.id, 1);
        assert_eq!(e.owner, CardId::new(0));
    }
}
