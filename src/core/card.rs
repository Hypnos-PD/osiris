use crate::core::enums::*;
use mlua::{UserData, UserDataMethods};
use crate::core::types::CardId;

/// StatBlock stores a card's original/current mutable attributes
// Keep traits minimal to avoid relying on derived traits from bitflags
pub struct StatBlock {
    pub type_: CardType,
    pub level: u32,
    pub rank: u32,
    pub link: u32,
    pub code2: u32,
    pub lscale: u32,
    pub rscale: u32,
    pub attribute: CardAttribute,
    pub race: CardRace,
    pub attack: i32,
    pub defense: i32,
    pub base_attack: i32,
    pub base_defense: i32,
}

// Default derived above; custom default ensures bitflags are set to empty
impl Default for StatBlock {
    fn default() -> Self { StatBlock { type_: CardType::empty(), level: 0, rank: 0, link: 0, code2: 0, lscale: 0, rscale: 0, attribute: CardAttribute::empty(), race: CardRace::empty(), attack: 0, defense: 0, base_attack: 0, base_defense: 0 } }
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

    /// Mark the given status bits on the card.
    pub fn set_status(&mut self, status: CardStatus) {
        self.status |= status;
    }

    /// Clear the given status bits on the card.
    pub fn clear_status(&mut self, status: CardStatus) {
        self.status.remove(status);
    }

    /// Test whether the card has at least one of the `status` bits set.
    pub fn has_status(&self, status: CardStatus) -> bool {
        self.status.intersects(status)
    }
}

impl UserData for CardId {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        // Method: c:RegisterEffect(e) - stub for now
        methods.add_method_mut("RegisterEffect", |_, _self, _effect: mlua::AnyUserData| {
            // Stub - register effect on card
            Ok(())
        });
        
        // Method: c:GetCode() - returns card code
        methods.add_method("GetCode", |_, self_, ()| {
            // For now, return the CardId value as code
            // In reality, we'd need to look up the actual card code from the duel
            Ok(self_.0)
        });
        
        // Method: c:GetControler() - returns controller
        methods.add_method("GetControler", |_, _self, ()| {
            // Stub - return 0 for now
            Ok(0)
        });
        
        // Method: c:GetLocation() - returns location
        methods.add_method("GetLocation", |_, _self, ()| {
            // Stub - return 0 for now
            Ok(0)
        });
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

    #[test]
    fn status_helpers_work() {
        let mut c = Card::new(1);
        assert!(!c.has_status(CardStatus::DISABLED));
        c.set_status(CardStatus::DISABLED);
        assert!(c.has_status(CardStatus::DISABLED));
        c.clear_status(CardStatus::DISABLED);
        assert!(!c.has_status(CardStatus::DISABLED));
    }
}
