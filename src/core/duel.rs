use crate::core::card::Card;
use crate::core::enums::Location;
use crate::core::field::Field;
use crate::core::mtrandom::Mt19937;
use crate::core::chain::Chain;
use crate::core::scripting::{FileSystemLoader, ScriptLoader};
// import Effect type (may be used for future processor logic)
use crate::core::types::CardId;
use mlua::Lua;
use std::path::PathBuf;

/// Duel acts as the arena holding cards and the field.
pub struct Duel {
    pub cards: Vec<Card>,
    pub field: Field,
    pub random: Mt19937,
    pub chain: Chain,
    pub state: ProcessorState,
    pub turn: u32,
    pub turn_player: u8,
    pub lua: Lua,
}

impl Duel {
    pub fn new(seed: u32) -> Self {
        let lua = Lua::new();
        let mut duel = Duel { 
            cards: Vec::new(), 
            field: Field::new(), 
            random: Mt19937::new(seed),
            chain: Chain::new(),
            state: ProcessorState::Start,
            turn: 0,
            turn_player: 0,
            lua,
        };
        duel.load_core_scripts().expect("Failed to load core Lua scripts");
        duel
    }

    /// Load core Lua scripts (constant.lua and utility.lua) from the external YGOPro script directory.
    pub fn load_core_scripts(&mut self) -> mlua::Result<()> {
        let loader = FileSystemLoader::new(PathBuf::from("../external/ygopro/script"));
        
        // Load constant.lua
        let constant_script = loader.load_script("constant.lua")
            .ok_or(mlua::Error::RuntimeError("Failed to load constant.lua".to_string()))?;
        self.lua.load(&constant_script).exec()?;
        
        // Note: utility.lua requires additional dependencies that aren't available yet
        // We'll load it later when we have the full Lua environment set up
        // let utility_script = loader.load_script("utility.lua")
        //     .ok_or(mlua::Error::RuntimeError("Failed to load utility.lua".to_string()))?;
        // self.lua.load(&utility_script).exec()?;
        
        Ok(())
    }

}

/// ProcessorState is the high-level step used by the duel processor loop.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ProcessorState {
    Start,
    TurnChange,
    Draw,
    Standby,
    Main1,
    Battle,
    Main2,
    End,
    GameOver,
}

impl Duel {
    /// Process the duel state machine for one cycle: returns true to continue, false to stop.
    pub fn process(&mut self) -> bool {
        match self.state {
            ProcessorState::Start => {
                println!("Duel Start");
                // Shuffle and draw initial hands for both players
                for p in 0..2u8 {
                    self.shuffle_deck(p);
                }
                // Draw 5 for each player
                for p in 0..2u8 {
                    self.draw(p, 5);
                }
                self.state = ProcessorState::TurnChange;
                true
            }
            ProcessorState::TurnChange => {
                self.turn += 1;
                self.turn_player = (self.turn_player + 1) % 2;
                self.state = ProcessorState::Draw;
                true
            }
            ProcessorState::Draw => {
                self.draw(self.turn_player, 1);
                self.state = ProcessorState::Main1;
                true
            }
            ProcessorState::Main1 => {
                println!("Main Phase 1");
                // Stop processing until player inputs / network interaction
                false
            }
            _ => false,
        }
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
        // Precompute which type of location we're adding to (we'll move target_loc into add_card below)
        let is_deck = target_loc.contains(Location::DECK);
        let is_hand = target_loc.contains(Location::HAND);
        let is_grave = target_loc.contains(Location::GRAVE);
        let is_removed = target_loc.contains(Location::REMOVED);
        let is_extra = target_loc.contains(Location::EXTRA);
        if let Some(cmut) = self.get_card_mut(card_id) {
            cmut.controller = target_player;
            cmut.location = crate::core::enums::Location::from_bits_truncate(target_loc.bits());
            cmut.sequence = target_seq;
        }
        // Add to new location
        self.field.add_card(target_player, target_loc, card_id, target_seq);
        // If added to a stack (deck/hand/grave/remove/extra) update the sequence to the final appended index.
        if is_deck {
            let idx = (self.field.deck[target_player as usize].len() - 1) as u8;
            if let Some(cmut) = self.get_card_mut(card_id) {
                cmut.sequence = idx;
            }
        } else if is_hand {
            let idx = (self.field.hand[target_player as usize].len() - 1) as u8;
            if let Some(cmut) = self.get_card_mut(card_id) {
                cmut.sequence = idx;
            }
        } else if is_grave {
            let idx = (self.field.grave[target_player as usize].len() - 1) as u8;
            if let Some(cmut) = self.get_card_mut(card_id) {
                cmut.sequence = idx;
            }
        } else if is_removed {
            let idx = (self.field.remove[target_player as usize].len() - 1) as u8;
            if let Some(cmut) = self.get_card_mut(card_id) {
                cmut.sequence = idx;
            }
        } else if is_extra {
            let idx = (self.field.extra[target_player as usize].len() - 1) as u8;
            if let Some(cmut) = self.get_card_mut(card_id) {
                cmut.sequence = idx;
            }
        }
        true
    }

    /// Shuffle the player's deck using Fisher-Yates and the duel's MT19937 RNG.
    pub fn shuffle_deck(&mut self, player: u8) {
        let p = player as usize;
        let deck = &mut self.field.deck[p];
        if deck.len() <= 1 {
            return;
        }
        for i in (1..deck.len()).rev() {
            let j = (self.random.gen_u32() as usize) % (i + 1);
            deck.swap(i, j);
        }
    }

    /// Draw `count` cards from player's deck to their hand (append to hand).
    pub fn draw(&mut self, player: u8, count: u32) {
        for _ in 0..count {
            let p = player as usize;
            if self.field.deck[p].is_empty() {
                break;
            }
            // Top card is last element
            let card_id = *self.field.deck[p].last().unwrap();
            // Move to hand (move_card will remove from deck)
            let _ = self.move_card(card_id, player, Location::HAND, 0);
        }
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

    #[test]
    fn test_shuffle_deck_and_draw() {
        let mut d = Duel::new(42);
        // create 10 cards in player 0 deck
        for i in 0..10 {
            d.create_card(i + 1, 0);
        }
        let original: Vec<_> = d.field.deck[0].clone();
        d.shuffle_deck(0);
        let shuffled: Vec<_> = d.field.deck[0].clone();
        assert_eq!(original.len(), shuffled.len());
        // Assert that the order changed in at least one position (probabilistic but extremely unlikely to be same)
        let mut same = true;
        for i in 0..original.len() {
            if original[i] != shuffled[i] {
                same = false;
                break;
            }
        }
        assert!(!same, "shuffle should change deck order");

        // test draw
        let mut d2 = Duel::new(100);
        for i in 0..5 {
            d2.create_card(i + 1, 0);
        }
        d2.draw(0, 2);
        assert_eq!(d2.field.hand[0].len(), 2);
        assert_eq!(d2.field.deck[0].len(), 3);
    }

    #[test]
    fn test_game_flow_stub() {
        let mut d = Duel::new(1);
        // create enough cards in player decks
        for i in 0..10 {
            d.create_card(100 + i, 0);
            d.create_card(200 + i, 1);
        }
        // Ensure initial pointers
        assert_eq!(d.state, ProcessorState::Start);
        // Run the processor until it pauses
        while d.process() {}
        // After first full cycle, turn should be > 0
        assert!(d.turn > 0);
        // Confirm hand sizes: both initially 5 after Start, and turn player got +1 in Draw
        let other_player = (d.turn_player + 1) % 2;
        assert_eq!(d.field.hand[other_player as usize].len(), 5);
        assert_eq!(d.field.hand[d.turn_player as usize].len(), 6);
        // Ensure state is Main1 (processing paused)
        assert_eq!(d.state, ProcessorState::Main1);
    }

    #[test]
    fn test_lua_integration() {
        let duel = Duel::new(42);
        
        // Test that Lua environment is initialized and can access constants from constant.lua
        let result: mlua::Result<u32> = duel.lua.globals().get("TYPE_MONSTER");
        assert!(result.is_ok(), "TYPE_MONSTER should be defined in Lua environment");
        
        let type_monster = result.unwrap();
        assert_eq!(type_monster, 0x1, "TYPE_MONSTER should equal 0x1");
        
        // Test another constant
        let result: mlua::Result<u32> = duel.lua.globals().get("LOCATION_DECK");
        assert!(result.is_ok(), "LOCATION_DECK should be defined in Lua environment");
        
        let location_deck = result.unwrap();
        assert_eq!(location_deck, 0x1, "LOCATION_DECK should equal 0x1");
    }
}
