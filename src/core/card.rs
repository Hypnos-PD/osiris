use crate::core::enums::*;

/// StatBlock stores a card's original/current mutable attributes
// Keep traits minimal to avoid relying on derived traits from bitflags
pub struct StatBlock {
    pub type_: CardType,
    pub level: u32,
    pub rank: u32,
    pub link: u32,
    pub attribute: CardAttribute,
    pub race: CardRace,
    pub attack: i32,
    pub defense: i32,
}

// Default derived above; custom default ensures bitflags are set to empty
impl Default for StatBlock {
    fn default() -> Self { StatBlock { type_: CardType::empty(), level: 0, rank: 0, link: 0, attribute: CardAttribute::empty(), race: CardRace::empty(), attack: 0, defense: 0 } }
}

/// Card structure for osiris core. Does not hold references to other cards or duel.
// Keep derives minimal for the same reason as StatBlock
pub struct Card {
    // Identity
    pub code: u32,
    pub alias: u32,

    // Stats
    pub original_stats: StatBlock,
    pub current_stats: StatBlock,

    // State
    pub location: Location,
    pub sequence: u8,
    pub position: CardPosition,
    pub owner: u8,
    pub controller: u8,
    pub reason: u32,

    // Flags
    // Placeholder for now; we can implement CardStatus as bitflags later.
    pub status: CardStatus,
}

impl Card {
    /// Construct a new Card with minimal defaults but a given `code`.
    pub fn new(code: u32) -> Self {
        Card {
            code,
            alias: 0,
            original_stats: StatBlock::default(),
            current_stats: StatBlock::default(),
            location: Location::empty(),
            sequence: 0,
            position: CardPosition::empty(),
            owner: 0,
            controller: 0,
            reason: 0,
            status: CardStatus::empty(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn card_new_defaults() {
        let c = Card::new(123);
        assert_eq!(c.code, 123);
        assert_eq!(c.alias, 0);
        assert_eq!(c.location.bits(), 0);
        assert_eq!(c.current_stats.attack, 0);
    }
}
