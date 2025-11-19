use crate::core::types::CardId;
use crate::core::enums::Location;

/// Field stores lists of CardId for each player (0/1) and fixed-size zones.
pub struct Field {
    pub deck: [Vec<CardId>; 2],
    pub hand: [Vec<CardId>; 2],
    pub grave: [Vec<CardId>; 2],
    pub remove: [Vec<CardId>; 2],
    pub extra: [Vec<CardId>; 2],
    pub mzone: [[Option<CardId>; 7]; 2],
    pub szone: [[Option<CardId>; 8]; 2],
}

impl Field {
    pub fn new() -> Self {
        Field {
            deck: [Vec::new(), Vec::new()],
            hand: [Vec::new(), Vec::new()],
            grave: [Vec::new(), Vec::new()],
            remove: [Vec::new(), Vec::new()],
            extra: [Vec::new(), Vec::new()],
            mzone: [[None; 7], [None; 7]],
            szone: [[None; 8], [None; 8]],
        }
    }

    /// Add a card to the specified player/location/sequence.
    /// For stacks (deck/hand/grave/remove/extra) we append to the vector.
    /// For zones (mzone/szone) we place at the given sequence (index) and overwrite.
    pub fn add_card(&mut self, player: u8, location: Location, card: CardId, sequence: u8) {
        let p = player as usize;
        if location.contains(Location::DECK) {
            self.deck[p].push(card);
        } else if location.contains(Location::HAND) {
            self.hand[p].push(card);
        } else if location.contains(Location::GRAVE) {
            self.grave[p].push(card);
        } else if location.contains(Location::REMOVED) {
            self.remove[p].push(card);
        } else if location.contains(Location::EXTRA) {
            self.extra[p].push(card);
        } else if location.contains(Location::MZONE) {
            let idx = sequence as usize;
            if idx < self.mzone[p].len() {
                self.mzone[p][idx] = Some(card);
            } else {
                // ignore out-of-range placements silently for now
            }
        } else if location.contains(Location::SZONE) {
            let idx = sequence as usize;
            if idx < self.szone[p].len() {
                self.szone[p][idx] = Some(card);
            } else {
                // ignore out-of-range placements silently for now
            }
        }
    }

    /// Remove a card for a given player and location by sequence / index.
    /// For zones (mzone/szone) this takes the value at the sequence and returns it.
    /// For stacks (deck/hand/grave/remove/extra) this removes the card at the given sequence index if present.
    /// Returns Some(CardId) if found and removed, else None.
    pub fn remove_card(&mut self, player: u8, location: Location, sequence: u8) -> Option<CardId> {
        let p = player as usize;
        if location.contains(Location::DECK) {
            let idx = sequence as usize;
            if idx < self.deck[p].len() {
                return Some(self.deck[p].remove(idx));
            }
            return None;
        } else if location.contains(Location::HAND) {
            let idx = sequence as usize;
            if idx < self.hand[p].len() {
                return Some(self.hand[p].remove(idx));
            }
            return None;
        } else if location.contains(Location::GRAVE) {
            let idx = sequence as usize;
            if idx < self.grave[p].len() {
                return Some(self.grave[p].remove(idx));
            }
            return None;
        } else if location.contains(Location::REMOVED) {
            let idx = sequence as usize;
            if idx < self.remove[p].len() {
                return Some(self.remove[p].remove(idx));
            }
            return None;
        } else if location.contains(Location::EXTRA) {
            let idx = sequence as usize;
            if idx < self.extra[p].len() {
                return Some(self.extra[p].remove(idx));
            }
            return None;
        } else if location.contains(Location::MZONE) {
            // remove if occupies the zone
            let idx = sequence as usize;
            if idx < self.mzone[p].len() {
                return self.mzone[p][idx].take();
            }
            return None;
        } else if location.contains(Location::SZONE) {
            let idx = sequence as usize;
            if idx < self.szone[p].len() {
                return self.szone[p][idx].take();
            }
            return None;
        }
        None
    }

    /// Remove a card from a stacked location (DECK/HAND/GRAVE/REMOVED/EXTRA) by CardId.
    /// This searches for the matching CardId and removes it from the Vec, returning the CardId if found.
    pub fn remove_card_from_stack(&mut self, player: u8, location: Location, card: CardId) -> Option<CardId> {
        let p = player as usize;
        if location.contains(Location::DECK) {
            if let Some(idx) = self.deck[p].iter().position(|&c| c == card) {
                return Some(self.deck[p].remove(idx));
            }
            return None;
        } else if location.contains(Location::HAND) {
            if let Some(idx) = self.hand[p].iter().position(|&c| c == card) {
                return Some(self.hand[p].remove(idx));
            }
            return None;
        } else if location.contains(Location::GRAVE) {
            if let Some(idx) = self.grave[p].iter().position(|&c| c == card) {
                return Some(self.grave[p].remove(idx));
            }
            return None;
        } else if location.contains(Location::REMOVED) {
            if let Some(idx) = self.remove[p].iter().position(|&c| c == card) {
                return Some(self.remove[p].remove(idx));
            }
            return None;
        } else if location.contains(Location::EXTRA) {
            if let Some(idx) = self.extra[p].iter().position(|&c| c == card) {
                return Some(self.extra[p].remove(idx));
            }
            return None;
        }
        None
    }

    /// Find the first empty monster zone slot for a player
    pub fn find_empty_mzone_slot(&self, player: u8) -> Option<u8> {
        let p = player as usize;
        for (index, slot) in self.mzone[p].iter().enumerate() {
            if slot.is_none() {
                return Some(index as u8);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::CardId;
    use crate::core::enums::Location;

    #[test]
    fn field_new_empty() {
        let f = Field::new();
        assert_eq!(f.deck[0].len(), 0);
        assert_eq!(f.hand[1].len(), 0);
        assert!(f.mzone[0][0].is_none());
        assert!(f.szone[1][7].is_none());
        // add a card to deck 0
        let mut f2 = f;
        f2.deck[0].push(CardId::new(1));
        assert_eq!(f2.deck[0].len(), 1);
    }

    #[test]
    fn add_and_remove_deck() {
        let mut f = Field::new();
        let id = CardId::new(42);
        f.add_card(0, Location::DECK, id, 0);
        assert_eq!(f.deck[0].len(), 1);
        assert_eq!(f.deck[0][0], id);
        let r = f.remove_card_from_stack(0, Location::DECK, id).expect("should remove");
        assert_eq!(r, id);
        assert_eq!(f.deck[0].len(), 0);
    }

    #[test]
    fn add_and_remove_mzone() {
        let mut f = Field::new();
        let id = CardId::new(3);
        f.add_card(1, Location::MZONE, id, 0);
        assert!(f.mzone[1][0].is_some());
        assert_eq!(f.mzone[1][0].unwrap(), id);
        let r = f.remove_card(1, Location::MZONE, 0).expect("removed");
        assert_eq!(r, id);
        assert!(f.mzone[1][0].is_none());
    }

    #[test]
    fn find_empty_mzone_slot_works() {
        let mut f = Field::new();
        
        // Test empty field
        assert_eq!(f.find_empty_mzone_slot(0), Some(0));
        assert_eq!(f.find_empty_mzone_slot(1), Some(0));
        
        // Fill some slots
        f.mzone[0][0] = Some(CardId::new(1));
        f.mzone[0][1] = Some(CardId::new(2));
        
        assert_eq!(f.find_empty_mzone_slot(0), Some(2));
        assert_eq!(f.find_empty_mzone_slot(1), Some(0));
        
        // Fill all slots
        for i in 0..7 {
            f.mzone[0][i] = Some(CardId::new(i as u32 + 10));
        }
        
        assert_eq!(f.find_empty_mzone_slot(0), None);
    }
}
