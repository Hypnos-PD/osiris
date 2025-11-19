use crate::core::card::Card;
use crate::core::field::Field;
use crate::core::types::CardId;

/// Duel acts as the arena holding cards and the field.
pub struct Duel {
    pub cards: Vec<Card>,
    pub field: Field,
    pub random_seed: u32,
}

impl Duel {
    pub fn new(seed: u32) -> Self {
        Duel { cards: Vec::new(), field: Field::new(), random_seed: seed }
    }

    /// Create a card in the arena and return its CardId handle.
    pub fn create_card(&mut self, code: u32, owner: u8) -> CardId {
        let mut card = Card::new(code);
        card.owner = owner;
        card.controller = owner;
        // initialize original stats base values if needed
        card.original_stats.base_attack = card.original_stats.attack;
        card.original_stats.base_defense = card.original_stats.defense;
        // push and return index
        self.cards.push(card);
        CardId::new((self.cards.len() - 1) as u32)
    }

    pub fn get_card(&self, id: CardId) -> Option<&Card> {
        self.cards.get(id.0 as usize)
    }

    pub fn get_card_mut(&mut self, id: CardId) -> Option<&mut Card> {
        self.cards.get_mut(id.0 as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn create_card_assigns_index_owner() {
        let mut d = Duel::new(42);
        let id = d.create_card(12345, 1);
        assert_eq!(id.0, 0);
        let card = d.get_card(id).expect("card should exist");
        assert_eq!(card.code, 12345);
        assert_eq!(card.owner, 1);
    }

    #[test]
    fn create_card_seq_and_mut_access() {
        let mut d = Duel::new(1);
        let a = d.create_card(1, 0);
        let b = d.create_card(2, 1);
        assert_ne!(a.0, b.0);
        let mut_ref = d.get_card_mut(a).expect("exists");
        mut_ref.sequence = 5;
        assert_eq!(d.get_card(a).unwrap().sequence, 5);
    }
}
