use crate::core::card::Card;
use crate::core::enums::Location;
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
        let id = CardId::new((self.cards.len() - 1) as u32);
        // Put the card into the owner's deck by default
        let p = owner as usize;
        let seq = self.field.deck[p].len() as u8; // deck index pre-push
        // Note: field.add_card will push into deck and sequence will be adjusted
        self.field.add_card(owner, Location::DECK, id, seq);
        if let Some(card_mut) = self.get_card_mut(id) {
            card_mut.location = Location::DECK;
            card_mut.sequence = seq;
        }
        id
    }

    /// Move a card from its current field location to a new target location/sequence.
    /// Returns true if move is successful.
    pub fn move_card(&mut self, card_id: CardId, target_player: u8, target_loc: Location, target_seq: u8) -> bool {
        // Gather current card state (controller, location, sequence)
        let (cur_player, cur_loc, cur_seq) = {
            if let Some(c) = self.get_card(card_id) {
                (c.controller, crate::core::enums::Location::from_bits_truncate(c.location.bits()), c.sequence)
            } else {
                return false;
            }
        };
        // Remove from current location
        let removed = if cur_loc.contains(Location::MZONE) || cur_loc.contains(Location::SZONE) {
            // zones are removed by sequence index
            self.field.remove_card(cur_player, cur_loc, cur_seq)
        } else {
            // stacks are removed by CardId search
            self.field.remove_card_from_stack(cur_player, cur_loc, card_id)
        };
        if removed.is_none() {
            return false;
        }
        // Update card internal state
        // Clone target location for reuse (Location may not be Copy)
        let target_loc_clone = crate::core::enums::Location::from_bits_truncate(target_loc.bits());
        if let Some(cmut) = self.get_card_mut(card_id) {
            cmut.controller = target_player;
            cmut.location = crate::core::enums::Location::from_bits_truncate(target_loc_clone.bits());
            cmut.sequence = target_seq;
        }
        // Add to new location
        self.field.add_card(target_player, target_loc_clone, card_id, target_seq);
        true
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
    use crate::core::enums::Location;
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

    #[test]
    fn move_card_deck_to_hand_to_mzone() {
        let mut d = Duel::new(0);
        let id = d.create_card(1111, 0);
        // initial should be in deck
        let c = d.get_card(id).unwrap();
        assert!(c.location.contains(Location::DECK));
        assert_eq!(d.field.deck[0][0], id);

        // move to hand
        assert!(d.move_card(id, 0, Location::HAND, 0));
        let c2 = d.get_card(id).unwrap();
        assert!(c2.location.contains(Location::HAND));
        assert!(d.field.deck[0].is_empty());
        assert_eq!(d.field.hand[0][0], id);

        // move to mzone slot 0
        assert!(d.move_card(id, 0, Location::MZONE, 0));
        let c3 = d.get_card(id).unwrap();
        assert!(c3.location.contains(Location::MZONE));
        assert!(!d.field.hand[0].contains(&id));
        assert_eq!(d.field.mzone[0][0].unwrap(), id);
    }

    #[test]
    fn test_move_card_mechanics() {
        let mut d = Duel::new(0);
        let id = d.create_card(9999, 0);
        // start in deck
        assert!(d.get_card(id).unwrap().location.contains(Location::DECK));
        // move to hand
        assert!(d.move_card(id, 0, Location::HAND, 0));
        assert!(d.get_card(id).unwrap().location.contains(Location::HAND));
        assert_eq!(d.field.hand[0][0], id);
        // move from hand to mzone sequence 2
        assert!(d.move_card(id, 0, Location::MZONE, 2));
        assert!(d.get_card(id).unwrap().location.contains(Location::MZONE));
        assert_eq!(d.field.mzone[0][2].unwrap(), id);
        // ensure hand no longer contains the card
        assert!(!d.field.hand[0].contains(&id));
    }
}
