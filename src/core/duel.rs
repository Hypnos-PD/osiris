use crate::core::card::Card;
use crate::core::enums::{Location, CardStatus};
use crate::core::field::Field;
use crate::core::mtrandom::Mt19937;
use crate::core::chain::Chain;
use crate::core::scripting::{FileSystemLoader, ScriptLoader};
use crate::core::group::Group;
use crate::core::effect::Effect;
use crate::core::types::EffectId;
// import Effect type (may be used for future processor logic)
use crate::core::types::CardId;
use mlua::{Lua, UserData};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// DuelData holds all the game state that needs to be shared between the main loop and Lua callbacks.
pub struct DuelData {
    pub cards: Vec<Card>,
    pub field: Field,
    pub random: Mt19937,
    pub chain: Chain,
    pub state: ProcessorState,
    pub turn: u32,
    pub turn_player: u8,
    pub effects: Vec<Effect>,
    pub triggered_effects: Vec<EffectId>,
}

impl DuelData {
    /// Get a card by its ID
    pub fn get_card(&self, id: CardId) -> Option<Card> {
        let index = id.as_u32() as usize;
        if index < self.cards.len() {
            Some(self.cards[index].clone())
        } else {
            None
        }
    }

    /// Send a card to a specific location with a reason
    pub fn send_card_to(&mut self, card_id: CardId, target_player: u8, location: Location, reason: u32) -> bool {
        // First update the card's reason
        if let Some(card) = self.cards.get_mut(card_id.0 as usize) {
            card.reason = reason;
        }
        
        // Then move the card to the target location
        // For now, we'll use a simple move to the specified location
        // In a full implementation, we might need to handle special cases
        let target_seq = 0; // Default sequence for now
        
        // Use the field's move_card logic
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
        if let Some(cmut) = self.cards.get_mut(card_id.0 as usize) {
            cmut.controller = target_player;
            cmut.location = crate::core::enums::Location::from_bits_truncate(location.bits());
            cmut.sequence = target_seq;
        }
        
        // Add to new location
        self.field.add_card(target_player, location, card_id, target_seq);
        
        true
    }

    /// Register an effect in the DuelData arena and optionally attach it to a card.
    pub fn register_effect(&mut self, effect: Effect, owner_card: Option<CardId>) -> EffectId {
        self.effects.push(effect);
        let id = EffectId::new((self.effects.len() - 1) as u32);
        if let Some(card_id) = owner_card {
            if let Some(card) = self.cards.get_mut(card_id.0 as usize) {
                card.effects.push(id);
            }
        }
        id
    }

    /// Return a list of EffectId candidates whose codes match the given event code.
    pub fn get_matching_effects(&self, code: u32) -> Vec<EffectId> {
        let mut out = Vec::new();
        for (idx, effect) in self.effects.iter().enumerate() {
            if effect.code == code {
                out.push(EffectId::new(idx as u32));
            }
        }
        out
    }
}

// Implement UserData for DuelData to make it accessible from Lua
impl UserData for DuelData {}

/// Duel acts as the arena holding cards and the field.
pub struct Duel {
    pub data: Arc<Mutex<DuelData>>,
    pub lua: Lua,
}

impl Duel {
    pub fn new(seed: u32) -> Self {
        let lua = Lua::new();
        
        // Register global tables in Lua
        {
            let globals = lua.globals();
            
            // Register Group table
            let group_table = lua.create_table().expect("Failed to create Group table");
            group_table.set("CreateGroup", lua.create_function(|_, ()| {
                Ok(Group::new())
            }).expect("Failed to create CreateGroup function")).expect("Failed to set CreateGroup");
            globals.set("Group", group_table).expect("Failed to set Group table");
            
            // Register Effect table
            let effect_table = lua.create_table().expect("Failed to create Effect table");
            effect_table.set("CreateEffect", lua.create_function(|_, card_ud: Option<mlua::AnyUserData>| {
                // Convert the optional userdata to CardId if present
                let card_id_opt = if let Some(ud) = card_ud {
                    if let Ok(cid) = ud.borrow::<CardId>() {
                        Some(*cid)
                    } else {
                        None
                    }
                } else { None };
                Ok(Effect::create_effect(card_id_opt))
            }).expect("Failed to create CreateEffect function")).expect("Failed to set CreateEffect");
            globals.set("Effect", effect_table).expect("Failed to set Effect table");
            
            // Register Card table
            let card_table = lua.create_table().expect("Failed to create Card table");
            let card_metatable = lua.create_table().expect("Failed to create Card metatable");
            card_metatable.set("__call", lua.create_function(|_, (_, id): (mlua::Table, u32)| {
                Ok(CardId(id))
            }).expect("Failed to create Card constructor")).expect("Failed to set Card constructor");
            card_table.set_metatable(Some(card_metatable));
            globals.set("Card", card_table).expect("Failed to set Card table");
            
            // Register Duel table with methods
            let duel_table = lua.create_table().expect("Failed to create Duel table");
            duel_table.set("RegisterEffect", lua.create_function(|_, (_effect, _player): (mlua::AnyUserData, u32)| {
                // Stub - register effect
                Ok(())
            }).expect("Failed to create RegisterEffect function")).expect("Failed to set RegisterEffect");
            
            duel_table.set("LoadScript", lua.create_function(|_lua, _name: String| {
                // Stub - load script by name
                // In reality, this would load from the script directory
                Ok(())
            }).expect("Failed to create LoadScript function")).expect("Failed to set LoadScript");
            
            // Add SendtoGrave method
            duel_table.set("SendtoGrave", lua.create_function(|lua, (cards, reason): (mlua::Value, u32)| {
                // Get DuelData from Lua app data
                let data = lua.app_data_ref::<Arc<Mutex<DuelData>>>()
                    .expect("DuelData not found in Lua app data");
                let mut data_guard = data.lock().unwrap();
                
                let mut count = 0;
                
                // Handle both single Card and Group
                match cards {
                    mlua::Value::UserData(ud) => {
                        if let Ok(card_id) = ud.borrow::<CardId>() {
                            // Single card
                            if let Some(card) = data_guard.get_card(*card_id) {
                                let owner = card.owner;
                                if data_guard.send_card_to(*card_id, owner, Location::GRAVE, reason) {
                                    count += 1;
                                }
                            }
                        } else if let Ok(group) = ud.borrow::<Group>() {
                            // Group of cards
                            for &card_id in &group.0 {
                                if let Some(card) = data_guard.get_card(card_id) {
                                    let owner = card.owner;
                                    if data_guard.send_card_to(card_id, owner, Location::GRAVE, reason) {
                                        count += 1;
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        // Invalid parameter type
                        return Err(mlua::Error::RuntimeError("SendtoGrave: expected Card or Group".to_string()));
                    }
                }
                
                Ok(count)
            }).expect("Failed to create SendtoGrave function")).expect("Failed to set SendtoGrave");
            
            // Add Summon method
            duel_table.set("Summon", lua.create_function(|lua, (player, card, _ignore_count, _effect_ptr): (u32, mlua::AnyUserData, bool, mlua::Value)| {
                // Get DuelData from Lua app data
                let data = lua.app_data_ref::<Arc<Mutex<DuelData>>>()
                    .expect("DuelData not found in Lua app data");
                let mut data_guard = data.lock().unwrap();
                
                let card_id = card.borrow::<CardId>()?;
                let player = player as u8;
                
                // Find empty monster zone slot
                if let Some(_empty_slot) = data_guard.field.find_empty_mzone_slot(player) {
                    // Move card to monster zone
                    if data_guard.send_card_to(*card_id, player, Location::MZONE, 0) {
                        // Set summon status
                        if let Some(card_mut) = data_guard.cards.get_mut(card_id.0 as usize) {
                            card_mut.set_status(CardStatus::SUMMON_TURN);
                        }
                        // Raise summon success event with the card wrapped in a Group
                        let mut g = Group::new();
                        g.0.insert(*card_id);
                        // Release the lock before evaluating Lua conditions to avoid deadlocks
                        drop(data_guard);
                        // Call static raise_event handler which bridges Lua and DuelData
                        crate::core::duel::Duel::raise_event_static(lua, data.clone(), crate::core::enums::EVENT_SUMMON_SUCCESS, Some(g), player);
                        return Ok(1);
                    }
                }
                
                Ok(0) // Failed to summon
            }).expect("Failed to create Summon function")).expect("Failed to set Summon");
            
            globals.set("Duel", duel_table).expect("Failed to set Duel table");
        }
        
        let data = Arc::new(Mutex::new(DuelData {
            cards: Vec::new(),
            field: Field::new(),
            random: Mt19937::new(seed),
            chain: Chain::new(),
            state: ProcessorState::Start,
            turn: 0,
            turn_player: 0,
            effects: Vec::new(),
            triggered_effects: Vec::new(),
        }));
        
        // Inject state into Lua
        lua.set_app_data(data.clone());
        
        let mut duel = Duel {
            data,
            lua,
        };
        duel.load_core_scripts().expect("Failed to load core Lua scripts");
        duel
    }

    /// Resolve the chain: pop chain links in LIFO order and execute their operations.
    pub fn resolve_chain(&mut self) {
        loop {
            let next_link = {
                let mut data_guard = self.data.lock().unwrap();
                data_guard.chain.pop()
            };
            if let Some(link) = next_link {
                // Execute effect operation using saved effect id and trigger player
                let _ = Duel::execute_effect_static(&self.lua, self.data.clone(), link.effect_id, 0u32, link.target_cards, 0u8, link.trigger_player);
            } else {
                break;
            }
        }
    }

    /// Static helper to raise events from contexts where we only have Lua and access to the DuelData via app data.
    pub fn raise_event_static(lua: &Lua, data_arc: Arc<Mutex<DuelData>>, code: u32, _event_cards: Option<Group>, _reason_player: u8) {
        // Step 1: get list of candidate effect IDs from data
        let candidates = {
            let data_guard = data_arc.lock().unwrap();
            data_guard.get_matching_effects(code)
        };

        // Step 2: build vector of (EffectId, Option<Function>) so we can call them without holding the DuelData lock
        let mut callable: Vec<(EffectId, Option<mlua::Function>)> = Vec::new();
        for eid in candidates {
            let maybe_fn = {
                let data_guard = data_arc.lock().unwrap();
                if let Some(effect) = data_guard.effects.get(eid.0 as usize) {
                    if let Some(ref key) = effect.condition {
                        // Get the Function reference from the registry
                        match lua.registry_value::<mlua::Function>(key) {
                            Ok(f) => Some(f),
                            Err(_) => None,
                        }
                    } else { None }
                } else { None }
            };
            callable.push((eid, maybe_fn));
        }

        // Step 3: Call each condition function (or default true if none) and, on true, push to triggered_effects
        for (eid, maybe_fn) in callable {
            let result = if let Some(func) = maybe_fn {
                // Build args for condition function
                let args = (eid, 0u8, None::<Group>, 0u8, 0u32, None::<EffectId>, 0u32, 0u8);
                match func.call::<_, bool>(args) {
                    Ok(b) => b,
                    Err(_) => false,
                }
            } else {
                true // No condition => pass
            };
            if result {
                let mut data_guard = data_arc.lock().unwrap();
                // Record triggered effect and push to chain
                data_guard.triggered_effects.push(eid);
                let link = crate::core::chain::ChainLink { effect_id: eid, trigger_player: 0, target_cards: None };
                data_guard.chain.add(link);
            }
        }
    }

    /// Execute an effect's operation function.
    pub fn execute_effect_static(lua: &Lua, data_arc: Arc<Mutex<DuelData>>, effect_id: EffectId, _code: u32, event_cards: Option<Group>, reason_player: u8, trigger_player: u8) -> mlua::Result<()> {
        // Get the operation function if any
        let op_func = {
            let data_guard = data_arc.lock().unwrap();
            if let Some(effect) = data_guard.effects.get(effect_id.0 as usize) {
                if let Some(ref key) = effect.operation {
                    lua.registry_value::<mlua::Function>(key).ok()
                } else { None }
            } else { None }
        };

        if let Some(func) = op_func {
            // Build arguments tuple (e, tp, eg, ep, ev, re, r, rp)
            let args = (effect_id, trigger_player, event_cards, reason_player, 0u32, None::<EffectId>, 0u32, reason_player);
            // Call the operation function
            let _res: mlua::Value = func.call(args)?;
        }
        Ok(())
    }

    /// Load core Lua scripts (constant.lua, utility.lua, and procedure.lua) from the external YGOPro script directory.
    pub fn load_core_scripts(&mut self) -> mlua::Result<()> {
        let loader = FileSystemLoader::new(PathBuf::from("../external/ygopro/script"));
        
        // Load constant.lua
        let constant_script = loader.load_script("constant.lua")
            .ok_or(mlua::Error::RuntimeError("Failed to load constant.lua".to_string()))?;
        self.lua.load(&constant_script).exec()?;
        
        // Load utility.lua - now that Group is available
        let utility_script = loader.load_script("utility.lua")
            .ok_or(mlua::Error::RuntimeError("Failed to load utility.lua".to_string()))?;
        self.lua.load(&utility_script).exec()?;
        
        // Load procedure.lua - now that Effect and Card are available
        let procedure_script = loader.load_script("procedure.lua")
            .ok_or(mlua::Error::RuntimeError("Failed to load procedure.lua".to_string()))?;
        self.lua.load(&procedure_script).exec()?;
        
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
        let mut data = self.data.lock().unwrap();
        match data.state {
            ProcessorState::Start => {
                println!("Duel Start");
                // Shuffle and draw initial hands for both players
                for p in 0..2u8 {
                    self.shuffle_deck_internal(&mut data, p);
                }
                // Draw 5 for each player
                for p in 0..2u8 {
                    self.draw_internal(&mut data, p, 5);
                }
                data.state = ProcessorState::TurnChange;
                true
            }
            ProcessorState::TurnChange => {
                data.turn += 1;
                data.turn_player = (data.turn_player + 1) % 2;
                data.state = ProcessorState::Draw;
                true
            }
            ProcessorState::Draw => {
                // Release the lock before calling draw
                let turn_player = data.turn_player;
                drop(data);
                self.draw(turn_player, 1);
                let mut data = self.data.lock().unwrap();
                data.state = ProcessorState::Main1;
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
        let mut data = self.data.lock().unwrap();
        data.cards.push(card);
        let id = CardId::new((data.cards.len() - 1) as u32);
        // Put the card into the owner's deck by default
        let p = owner as usize;
        let seq = data.field.deck[p].len() as u8; // deck index pre-push
        // Note: field.add_card will push into deck and sequence will be adjusted
        data.field.add_card(owner, Location::DECK, id, seq);
        if let Some(card_mut) = data.cards.get_mut(id.0 as usize) {
            card_mut.location = Location::DECK;
            card_mut.sequence = seq;
        }
        id
    }

    /// Move a card from its current field location to a new target location/sequence.
    /// Returns true if move is successful.
    pub fn move_card(&mut self, card_id: CardId, target_player: u8, target_loc: Location, target_seq: u8) -> bool {
        let mut data = self.data.lock().unwrap();
        self.move_card_internal(&mut data, card_id, target_player, target_loc, target_seq)
    }

    fn move_card_internal(&self, data: &mut DuelData, card_id: CardId, target_player: u8, target_loc: Location, target_seq: u8) -> bool {
        // Gather current card state (controller, location, sequence)
        let (cur_player, cur_loc, cur_seq) = {
            if let Some(c) = data.get_card(card_id) {
                (c.controller, crate::core::enums::Location::from_bits_truncate(c.location.bits()), c.sequence)
            } else {
                return false;
            }
        };
        // Remove from current location
        let removed = if cur_loc.contains(Location::MZONE) || cur_loc.contains(Location::SZONE) {
            // zones are removed by sequence index
            data.field.remove_card(cur_player, cur_loc, cur_seq)
        } else {
            // stacks are removed by CardId search
            data.field.remove_card_from_stack(cur_player, cur_loc, card_id)
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
        if let Some(cmut) = data.cards.get_mut(card_id.0 as usize) {
            cmut.controller = target_player;
            cmut.location = crate::core::enums::Location::from_bits_truncate(target_loc.bits());
            cmut.sequence = target_seq;
        }
        // Add to new location
        data.field.add_card(target_player, target_loc, card_id, target_seq);
        // If added to a stack (deck/hand/grave/remove/extra) update the sequence to the final appended index.
        if is_deck {
            let idx = (data.field.deck[target_player as usize].len() - 1) as u8;
            if let Some(cmut) = data.cards.get_mut(card_id.0 as usize) {
                cmut.sequence = idx;
            }
        } else if is_hand {
            let idx = (data.field.hand[target_player as usize].len() - 1) as u8;
            if let Some(cmut) = data.cards.get_mut(card_id.0 as usize) {
                cmut.sequence = idx;
            }
        } else if is_grave {
            let idx = (data.field.grave[target_player as usize].len() - 1) as u8;
            if let Some(cmut) = data.cards.get_mut(card_id.0 as usize) {
                cmut.sequence = idx;
            }
        } else if is_removed {
            let idx = (data.field.remove[target_player as usize].len() - 1) as u8;
            if let Some(cmut) = data.cards.get_mut(card_id.0 as usize) {
                cmut.sequence = idx;
            }
        } else if is_extra {
            let idx = (data.field.extra[target_player as usize].len() - 1) as u8;
            if let Some(cmut) = data.cards.get_mut(card_id.0 as usize) {
                cmut.sequence = idx;
            }
        }
        true
    }

    /// Shuffle the player's deck using Fisher-Yates and the duel's MT19937 RNG.
    pub fn shuffle_deck(&mut self, player: u8) {
        let mut data = self.data.lock().unwrap();
        self.shuffle_deck_internal(&mut data, player);
    }

    fn shuffle_deck_internal(&self, data: &mut DuelData, player: u8) {
        let p = player as usize;
        let deck = &mut data.field.deck[p];
        if deck.len() <= 1 {
            return;
        }
        for i in (1..deck.len()).rev() {
            let j = (data.random.gen_u32() as usize) % (i + 1);
            deck.swap(i, j);
        }
    }

    /// Draw `count` cards from player's deck to their hand (append to hand).
    pub fn draw(&mut self, player: u8, count: u32) {
        for _ in 0..count {
            let data = self.data.lock().unwrap();
            let p = player as usize;
            if data.field.deck[p].is_empty() {
                break;
            }
            // Top card is last element
            let card_id = *data.field.deck[p].last().unwrap();
            // Move to hand (move_card will remove from deck)
            drop(data); // Release the lock before calling move_card
            let _ = self.move_card(card_id, player, Location::HAND, 0);
        }
    }

    fn draw_internal(&self, data: &mut DuelData, player: u8, count: u32) {
        for _ in 0..count {
            let p = player as usize;
            if data.field.deck[p].is_empty() {
                break;
            }
            // Top card is last element
            let card_id = *data.field.deck[p].last().unwrap();
            // Move to hand using internal move
            let _ = self.move_card_internal(data, card_id, player, Location::HAND, 0);
        }
    }

    // Note: get_card and get_card_mut are now available through DuelData::get_card
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
        let data = d.data.lock().unwrap();
        let card = data.get_card(id).expect("card should exist");
        assert_eq!(card.code, 12345);
        assert_eq!(card.owner, 1);
    }

    #[test]
    fn create_card_seq_and_mut_access() {
        let mut d = Duel::new(1);
        let a = d.create_card(1, 0);
        let b = d.create_card(2, 1);
        assert_ne!(a.0, b.0);
        let mut data = d.data.lock().unwrap();
        let mut_ref = data.cards.get_mut(a.0 as usize).expect("exists");
        mut_ref.sequence = 5;
        assert_eq!(data.get_card(a).unwrap().sequence, 5);
    }

    #[test]
    fn move_card_deck_to_hand_to_mzone() {
        let mut d = Duel::new(0);
        let id = d.create_card(1111, 0);
        // initial should be in deck
        let data = d.data.lock().unwrap();
        let c = data.get_card(id).unwrap();
        assert!(c.location.contains(Location::DECK));
        assert_eq!(data.field.deck[0][0], id);
        drop(data);

        // move to hand
        assert!(d.move_card(id, 0, Location::HAND, 0));
        let data = d.data.lock().unwrap();
        let c2 = data.get_card(id).unwrap();
        assert!(c2.location.contains(Location::HAND));
        assert!(data.field.deck[0].is_empty());
        assert_eq!(data.field.hand[0][0], id);
        drop(data);

        // move to mzone slot 0
        assert!(d.move_card(id, 0, Location::MZONE, 0));
        let data = d.data.lock().unwrap();
        let c3 = data.get_card(id).unwrap();
        assert!(c3.location.contains(Location::MZONE));
        assert!(!data.field.hand[0].contains(&id));
        assert_eq!(data.field.mzone[0][0].unwrap(), id);
    }

    #[test]
    fn test_move_card_mechanics() {
        let mut d = Duel::new(0);
        let id = d.create_card(9999, 0);
        // start in deck
        let data = d.data.lock().unwrap();
        assert!(data.get_card(id).unwrap().location.contains(Location::DECK));
        drop(data);
        // move to hand
        assert!(d.move_card(id, 0, Location::HAND, 0));
        let data = d.data.lock().unwrap();
        assert!(data.get_card(id).unwrap().location.contains(Location::HAND));
        assert_eq!(data.field.hand[0][0], id);
        drop(data);
        // move from hand to mzone sequence 2
        assert!(d.move_card(id, 0, Location::MZONE, 2));
        let data = d.data.lock().unwrap();
        assert!(data.get_card(id).unwrap().location.contains(Location::MZONE));
        assert_eq!(data.field.mzone[0][2].unwrap(), id);
        // ensure hand no longer contains the card
        assert!(!data.field.hand[0].contains(&id));
    }

    #[test]
    fn test_shuffle_deck_and_draw() {
        let mut d = Duel::new(42);
        // create 10 cards in player 0 deck
        for i in 0..10 {
            d.create_card(i + 1, 0);
        }
        let data = d.data.lock().unwrap();
        let original: Vec<_> = data.field.deck[0].clone();
        drop(data);
        d.shuffle_deck(0);
        let data = d.data.lock().unwrap();
        let shuffled: Vec<_> = data.field.deck[0].clone();
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
        drop(data);

        // test draw
        let mut d2 = Duel::new(100);
        for i in 0..5 {
            d2.create_card(i + 1, 0);
        }
        d2.draw(0, 2);
        let data = d2.data.lock().unwrap();
        assert_eq!(data.field.hand[0].len(), 2);
        assert_eq!(data.field.deck[0].len(), 3);
    }

    #[test]
    fn test_game_flow_stub() {
        let mut d = Duel::new(1);
        // create enough cards in player decks
        for i in 0..10 {
            d.create_card(100 + i, 0);
            d.create_card(200 + i, 1);
        }
        // Ensure initial pointers - state should be Start
        let data = d.data.lock().unwrap();
        println!("Initial state: {:?}", data.state);
        assert_eq!(data.state, ProcessorState::Start);
        drop(data);
        // Run the processor until it pauses
        while d.process() {}
        // After first full cycle, turn should be > 0
        let data = d.data.lock().unwrap();
        assert!(data.turn > 0);
        // Confirm hand sizes: both initially 5 after Start, and turn player got +1 in Draw
        let other_player = (data.turn_player + 1) % 2;
        assert_eq!(data.field.hand[other_player as usize].len(), 5);
        assert_eq!(data.field.hand[data.turn_player as usize].len(), 6);
        // Ensure state is Main1 (processing paused)
        assert_eq!(data.state, ProcessorState::Main1);
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

    #[test]
    fn test_lua_group() {
        let duel = Duel::new(42);
        
        // Test Group creation and basic functionality
        let result: mlua::Result<u32> = duel.lua.load(r#"
            local g = Group.CreateGroup()
            return g:GetCount()
        "#).eval();
        
        assert!(result.is_ok(), "Group creation and GetCount should work");
        let count = result.unwrap();
        assert_eq!(count, 0, "New group should have 0 cards");
        
        // Test the len meta method (#g)
        let result: mlua::Result<u32> = duel.lua.load(r#"
            local g = Group.CreateGroup()
            return #g
        "#).eval();
        
        assert!(result.is_ok(), "Group len meta method should work");
        let len = result.unwrap();
        assert_eq!(len, 0, "New group should have length 0");
    }

    #[test]
    fn test_lua_effect_creation() {
        let duel = Duel::new(42);
        
        // Test Effect creation
        let result: mlua::Result<mlua::AnyUserData> = duel.lua.load(r#"
            local e = Effect.CreateEffect(nil)
            return e
        "#).eval();
        
        assert!(result.is_ok(), "Effect creation should work");
        let effect = result.unwrap();
        assert!(effect.is::<Effect>(), "Created object should be an Effect");
    }

    #[test]
    fn test_lua_procedure_script_loaded() {
        let duel = Duel::new(42);
        
        // Test that procedure.lua is loaded by checking for the Auxiliary table it defines
        // procedure.lua should define various helper functions in the Auxiliary table
        let result: mlua::Result<mlua::Table> = duel.lua.globals().get("Auxiliary");
        
        // Auxiliary is a table defined in procedure.lua
        // If it exists, procedure.lua loaded successfully
        assert!(result.is_ok(), "procedure.lua should be loaded and define Auxiliary table");
    }

    #[test]
    fn test_lua_card_api_returns_real_data() {
        let mut duel = Duel::new(42);
        
        // Create a card with specific properties
        let card_id = duel.create_card(12345, 0);
        
        // Move the card to hand and set controller
        duel.move_card(card_id, 0, Location::HAND, 0);
        
        // Access the card through Lua API to verify it returns real data
        let result: mlua::Result<u32> = duel.lua.load(format!(
            r#"
            local card = Card({})
            return card:GetCode()
            "#,
            card_id.0
        )).eval();
        
        if let Err(e) = &result {
            println!("Lua error: {}", e);
        }
        assert!(result.is_ok(), "Card:GetCode() should work");
        let code = result.unwrap();
        assert_eq!(code, 12345, "Card:GetCode() should return the actual card code");
        
        // Test GetControler
        let result: mlua::Result<u32> = duel.lua.load(format!(
            r#"
            local card = Card({})
            return card:GetControler()
            "#,
            card_id.0
        )).eval();
        
        assert!(result.is_ok(), "Card:GetControler() should work");
        let controller = result.unwrap();
        assert_eq!(controller, 0, "Card:GetControler() should return the actual controller");
        
        // Test GetLocation
        let result: mlua::Result<u32> = duel.lua.load(format!(
            r#"
            local card = Card({})
            return card:GetLocation()
            "#,
            card_id.0
        )).eval();
        
        assert!(result.is_ok(), "Card:GetLocation() should work");
        let location = result.unwrap();
        assert_eq!(location, Location::HAND.bits() as u32, "Card:GetLocation() should return the actual location");
    }

    #[test]
    fn test_lua_actions() {
        let mut duel = Duel::new(42);
        
        // Create a card and add it to hand
        let card_id = duel.create_card(12345, 0);
        duel.move_card(card_id, 0, Location::HAND, 0);
        
        // Verify card is initially in hand
        {
            let data = duel.data.lock().unwrap();
            if let Some(card) = data.get_card(card_id) {
                assert!(card.location.contains(Location::HAND), "Card should start in hand");
            }
        }
        
        // Test SendtoGrave - send card from hand to grave
        let result: mlua::Result<u32> = duel.lua.load(format!(
            r#"
            local c = Card({})
            return Duel.SendtoGrave(c, 0x1)  -- REASON_EFFECT
            "#,
            card_id.0
        )).eval();
        
        assert!(result.is_ok(), "Duel.SendtoGrave should work");
        let count = result.unwrap();
        assert_eq!(count, 1, "SendtoGrave should return 1 for successful move");
        
        // Verify card is now in grave
        {
            let data = duel.data.lock().unwrap();
            if let Some(card) = data.get_card(card_id) {
                assert!(card.location.contains(Location::GRAVE), "Card should be in grave after SendtoGrave");
                assert_eq!(card.reason, 0x1, "Card reason should be set to REASON_EFFECT");
            }
            assert!(data.field.grave[0].contains(&card_id), "Card should be in player 0's grave");
        }
        
        // Test Summon - summon card from grave to monster zone
        let result: mlua::Result<u32> = duel.lua.load(format!(
            r#"
            local c = Card({})
            return Duel.Summon(0, c, false, nil)
            "#,
            card_id.0
        )).eval();
        
        assert!(result.is_ok(), "Duel.Summon should work");
        let success = result.unwrap();
        assert_eq!(success, 1, "Summon should return 1 for successful summon");
        
        // Verify card is now in monster zone with summon status
        {
            let data = duel.data.lock().unwrap();
            if let Some(card) = data.get_card(card_id) {
                assert!(card.location.contains(Location::MZONE), "Card should be in monster zone after Summon");
                assert!(card.has_status(CardStatus::SUMMON_TURN), "Card should have SUMMON_TURN status");
            }
            // Check that card is in monster zone
            let mut found = false;
            for slot in &data.field.mzone[0] {
                if let Some(id) = slot {
                    if *id == card_id {
                        found = true;
                        break;
                    }
                }
            }
            assert!(found, "Card should be in player 0's monster zone");
        }
    }

    #[test]
    fn test_effect_registration() {
        let mut duel = Duel::new(42);
        let card_id = duel.create_card(123, 0);
        duel.move_card(card_id, 0, Location::HAND, 0);

        // Execute Lua script to create and register an effect on the card
        let result: mlua::Result<()> = duel.lua.load(format!(r#"
            local c = Card({})
            local e = Effect.CreateEffect(c)
            e:SetCode(123)
            c:RegisterEffect(e)
            return nil
        "#, card_id.0)).exec();
        if let Err(e) = &result {
            println!("Lua error: {}", e);
        }
        assert!(result.is_ok(), "Lua script to register effect should run");

        // Verify the Card has the effect id
        let data = duel.data.lock().unwrap();
        let c = data.get_card(card_id).unwrap();
        assert!(!c.effects.is_empty(), "Card effects should contain the registered effect");
        // Verify the effect is in the duel effects arena with correct code
        assert!(data.effects.len() > 0, "Duel effects arena should have at least one effect");
        assert_eq!(data.effects[0].code, 123);
    }

    #[test]
    fn test_event_trigger() {
        let mut duel = Duel::new(42);
        let card_id = duel.create_card(400, 0);
        duel.move_card(card_id, 0, Location::HAND, 0);

        // Register an effect that triggers on summon success
        let result: mlua::Result<()> = duel.lua.load(format!(r#"
            local c = Card({})
            local e = Effect.CreateEffect(c)
            e:SetCode(0x1000)
            c:RegisterEffect(e)
            return nil
        "#, card_id.0)).exec();
        assert!(result.is_ok(), "Lua script to register effect should run");

        // Ensure no triggers before summon
        {
            let data = duel.data.lock().unwrap();
            assert!(data.triggered_effects.is_empty(), "No triggered effects initially");
        }

        // Summon the card (should trigger event)
        let result: mlua::Result<u32> = duel.lua.load(format!(
            r#"
            local c = Card({})
            return Duel.Summon(0, c, false, nil)
            "#,
            card_id.0
        )).eval();
        assert!(result.is_ok(), "Duel.Summon should work");

        // Verify trigger recorded
        {
            let data = duel.data.lock().unwrap();
            let c = data.get_card(card_id).unwrap();
            assert!(!c.effects.is_empty(), "Card has effect attached");
            let eid = c.effects[0];
            assert!(data.triggered_effects.contains(&eid), "Effect should have been triggered by the event");
            // Also verify arena effect code
            assert_eq!(data.effects[eid.0 as usize].code, 0x1000);
        }
    }

    #[test]
    fn test_condition_logic() {
        let mut duel = Duel::new(42);
        let card_id = duel.create_card(900, 0);
        duel.move_card(card_id, 0, Location::HAND, 0);

        // Register an effect that returns false
        let script_false = format!(r#"
            local c = Card({})
            local e = Effect.CreateEffect(c)
            e:SetCode(0x1000)
            e:SetCondition(function() return false end)
            c:RegisterEffect(e)
            return nil
        "#, card_id.0);
        let result: mlua::Result<()> = duel.lua.load(&script_false).exec();
        assert!(result.is_ok(), "Lua script to register false effect should run");

        // Register an effect that returns true
        let script_true = format!(r#"
            local c = Card({})
            local e = Effect.CreateEffect(c)
            e:SetCode(0x1000)
            e:SetCondition(function() return true end)
            c:RegisterEffect(e)
            return nil
        "#, card_id.0);
        let result2: mlua::Result<()> = duel.lua.load(&script_true).exec();
        assert!(result2.is_ok(), "Lua script to register true effect should run");

        // Ensure no triggers before summon
        {
            let data = duel.data.lock().unwrap();
            assert!(data.triggered_effects.is_empty(), "No triggered effects initially");
        }

        // Summon the card (should trigger only the one with true condition)
        let result: mlua::Result<u32> = duel.lua.load(format!(
            r#"
            local c = Card({})
            return Duel.Summon(0, c, false, nil)
            "#,
            card_id.0
        )).eval();
        assert!(result.is_ok(), "Duel.Summon should work");

        // Verify triggers: only the true condition (second effect) should have been triggered
        {
            let data = duel.data.lock().unwrap();
            assert_eq!(data.triggered_effects.len(), 1, "Only one effect should be triggered");
            let triggered = data.triggered_effects[0];
            assert_eq!(triggered.0, 1, "Second registered effect (id=1) should be triggered");
        }
    }

    #[test]
    fn test_effect_resolution() {
        let mut duel = Duel::new(42);
        let card_id = duel.create_card(777, 0);
        duel.move_card(card_id, 0, Location::HAND, 0);

        // Register an effect which sends its handler card to grave in operation
        let script = format!(r#"
            local c = Card({})
            local e = Effect.CreateEffect(c)
            e:SetCode(0x1000)
            e:SetCondition(function() return true end)
            e:SetOperation(function(e, tp, eg, ep, ev, re, r, rp)
                local c = e:GetHandler()
                Duel.SendtoGrave(c, 0x1)
            end)
            c:RegisterEffect(e)
            return nil
        "#, card_id.0);
        let result: mlua::Result<()> = duel.lua.load(&script).exec();
        assert!(result.is_ok(), "Lua script to register effect should run");

        // Summon the card which should trigger and push to chain (not execute yet)
        let res: mlua::Result<u32> = duel.lua.load(format!(r#"
            local c = Card({})
            return Duel.Summon(0, c, false, nil)
        "#, card_id.0)).eval();
        assert!(res.is_ok(), "Duel.Summon should work");

        // It should have been added to the chain but not executed yet
        {
            let data = duel.data.lock().unwrap();
            assert_eq!(data.chain.links.len(), 1, "Chain should have one link");
        }

        // Resolve the chain and then verify the operation executed
        duel.resolve_chain();
        let data = duel.data.lock().unwrap();
        if let Some(c) = data.get_card(card_id) {
            assert!(c.location.contains(Location::GRAVE), "Card should be in grave after operation executed");
        } else {
            panic!("card consistently missing");
        }
    }
}
