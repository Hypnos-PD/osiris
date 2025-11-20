use crate::core::card::Card;
use crate::core::enums::{Location, CardStatus};
use crate::core::field::Field;
use crate::core::mtrandom::Mt19937;
use crate::core::chain::Chain;
use crate::core::scripting::{FileSystemLoader, ScriptLoader};
use crate::core::group::Group;
use crate::core::effect::Effect;
use crate::core::types::EffectId;
use crate::core::database::Database;
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
    pub lp: [u32; 2],
    pub effects: Vec<Effect>,
    pub triggered_effects: Vec<EffectId>,
    pub database: std::sync::Arc<std::sync::Mutex<Database>>,
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

    /// Shuffle the specified player's deck.
    pub fn shuffle_deck(&mut self, player: u8) {
        self.shuffle_deck_internal(player);
    }

    fn shuffle_deck_internal(&mut self, player: u8) {
        let p = player as usize;
        self.random.shuffle_vector(&mut self.field.deck[p], 0, usize::MAX);
    }

    /// Draw count cards from player's deck to hand.
    pub fn draw(&mut self, player: u8, count: u32) {
        for _ in 0..count {
            let p = player as usize;
            if self.field.deck[p].is_empty() {
                break;
            }
            let card_id = self.field.deck[p].remove(0);
            self.field.hand[p].push(card_id);
            if let Some(card) = self.cards.get_mut(card_id.0 as usize) {
                card.location = Location::HAND;
                card.sequence = (self.field.hand[p].len() - 1) as u8;
            }
        }
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
    /// Load a deck for a player, clearing existing cards and creating new ones with proper location assignment.
    /// Main deck cards go to LOCATION_DECK, extra deck cards go to LOCATION_EXTRA.
    pub fn load_deck(&mut self, player: u8, main: &[u32], extra: &[u32]) {
        let mut data = self.data.lock().unwrap();
        
        // Clear existing decks for this player
        data.field.deck[player as usize].clear();
        data.field.extra[player as usize].clear();
        
        // Create main deck cards
        for &code in main.iter() {
            drop(data);
            let card_id = self.create_card(code, player);
            data = self.data.lock().unwrap();
            
            // Card is already in deck from create_card, just ensure location is correct
            if let Some(card) = data.cards.get_mut(card_id.0 as usize) {
                card.location = Location::DECK;
            }
        }
        
        // Create extra deck cards
        for &code in extra.iter() {
            drop(data);
            let card_id = self.create_card(code, player);
            data = self.data.lock().unwrap();
            
            // Move card from deck to extra deck
            // First remove from deck
            if let Some(pos) = data.field.deck[player as usize].iter().position(|&id| id == card_id) {
                data.field.deck[player as usize].remove(pos);
            }
            
            // Then add to extra deck
            let player_idx = player as usize;
            let seq = data.field.extra[player_idx].len() as u8;
            data.field.add_card(player, Location::EXTRA, card_id, seq);
            
            // Update card location and sequence
            if let Some(card) = data.cards.get_mut(card_id.0 as usize) {
                card.location = Location::EXTRA;
                card.sequence = seq;
            }
        }
    }

    /// Load a replay into the duel state (seed and decks). This is a stub and will not handle action replaying.
    pub fn load_replay(&mut self, replay: crate::core::replay::Replay) {
        let mut data = self.data.lock().unwrap();
        // Reset RNG using the header seed
        data.random = Mt19937::new(replay.header.seed);
        // Reset processor state to Start
        data.state = ProcessorState::Start;
        // Reset turn counter
        data.turn = 0;
        // Set starting LP according to replay parameters if provided
        let mut start_lp = 8000u32;
        if replay.params.start_lp > 0 {
            start_lp = replay.params.start_lp as u32;
        }
        data.lp = [start_lp, start_lp];
        drop(data);
        
        // Load decks for each player
        for (p_idx, deck) in replay.decks.iter().enumerate() {
            let p = p_idx as u8;
            self.load_deck(p, &deck.main, &deck.extra);
        }
    }
    /// Handle Start processing step (shuffle & initial draw)
    fn process_start(&mut self) -> bool {
        let mut data = self.data.lock().unwrap();
        println!("Duel Start");
        // Initialize LP to defaults for a duel start in case not set by load_replay
        data.lp = [8000, 8000];
        // Shuffle and draw are done by script in utility.lua BeginDuel
        data.state = ProcessorState::TurnChange;
        true
    }

    /// Handle turn change: increment turn, swap player, and go to Draw phase
    fn process_turn_change(&mut self) -> bool {
        let mut data = self.data.lock().unwrap();
        data.turn += 1;
        // On the first turn, choose starting player based on RNG so it matches C++'s selection.
        if data.turn == 1 {
            let v = data.random.gen_u32();
            data.turn_player = (v % 2) as u8;
        } else {
            data.turn_player = (data.turn_player + 1) % 2;
        }
        data.state = ProcessorState::Draw;
        true
    }

    /// Handle draw phase
    fn process_draw_phase(&mut self) -> bool {
        let turn_player = {
            let data = self.data.lock().unwrap();
            data.turn_player
        };
        // Draw a card for the current player; use `draw` which does lock management
        self.draw(turn_player, 1);
        // Raise EVENT_DRAW
        let lua = &self.lua;
        let data_arc = self.data.clone();
        Duel::raise_event_static(lua, data_arc, crate::core::enums::EVENT_DRAW, None, turn_player, None);
        // Go to Main1 phase next
        let mut data = self.data.lock().unwrap();
        data.state = ProcessorState::Main1;
        true
    }

    /// Handle main phases (Main1/Main2)
    fn process_main_phase(&mut self, _phase: crate::core::enums::Phase) -> bool {
        println!("Entering Main Phase");
        // For now, just wait for player input (return false to pause processing)
        false
    }
    pub fn new(seed: u32) -> Self {
        // default to creating an in-memory DB
        let db = Database::open_in_memory().expect("Failed to open default database");
        let db_arc = Arc::new(Mutex::new(db));
        Duel::new_with_db(seed, db_arc)
    }

    pub fn new_with_db(seed: u32, db_arc: Arc<Mutex<Database>>) -> Self {
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
                        crate::core::duel::Duel::raise_event_static(lua, data.clone(), crate::core::enums::EVENT_SUMMON_SUCCESS, Some(g), player, None);
                        return Ok(1);
                    }
                }
                
                Ok(0) // Failed to summon
            }).expect("Failed to create Summon function")).expect("Failed to set Summon");
            
            // Add ShuffleDeck method
            duel_table.set("ShuffleDeck", lua.create_function(|lua, player: u32| {
                let data = lua.app_data_ref::<Arc<Mutex<DuelData>>>()
                    .expect("DuelData not found in Lua app data");
                let mut data_guard = data.lock().unwrap();
                data_guard.shuffle_deck(player as u8);
                Ok(())
            }).expect("Failed to create ShuffleDeck function")).expect("Failed to set ShuffleDeck");
            
            // Add Draw method
            duel_table.set("Draw", lua.create_function(|lua, (player, count): (u32, u32)| {
                let data = lua.app_data_ref::<Arc<Mutex<DuelData>>>()
                    .expect("DuelData not found in Lua app data");
                let mut data_guard = data.lock().unwrap();
                data_guard.draw(player as u8, count);
                Ok(())
            }).expect("Failed to create Draw function")).expect("Failed to set Draw");
            
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
            lp: [8000, 8000],
            effects: Vec::new(),
            triggered_effects: Vec::new(),
                database: db_arc,
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
                let _ = Duel::execute_effect_static(&self.lua, self.data.clone(), link.effect_id, 0u32, link.target_cards, link.reason_effect, link.reason_player, link.trigger_player);
            } else {
                break;
            }
        }
    }

    /// Static helper to raise events from contexts where we only have Lua and access to the DuelData via app data.
    pub fn raise_event_static(lua: &Lua, data_arc: Arc<Mutex<DuelData>>, code: u32, event_cards: Option<Group>, reason_player: u8, reason_effect: Option<EffectId>) {
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
                let args = Duel::get_lua_args(lua, eid, reason_player, &event_cards, reason_player, reason_effect).expect("Failed to build args");
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
                let link = crate::core::chain::ChainLink { effect_id: eid, trigger_player: 0, target_cards: event_cards.clone(), reason_effect: reason_effect, reason_player };
                data_guard.chain.add(link);
            }
        }
    }

    /// Helper to produce Lua argument tuple (e, tp, eg, ep, ev, re, r, rp)
    pub fn get_lua_args<'lua>(_lua: &'lua Lua, effect_id: EffectId, trigger_player: u8, event_cards: &Option<Group>, reason_player: u8, reason_effect: Option<EffectId>) -> mlua::Result<(EffectId, u8, Option<Group>, u8, u32, Option<EffectId>, u32, u8)> {
        // We can copy Group and Option<EffectId> directly as they are ToLua.
        let eg = event_cards.clone();
        let re = reason_effect;
        let args = (effect_id, trigger_player, eg, reason_player, 0u32, re, 0u32, reason_player);
        Ok(args)
    }

    /// Execute an effect's operation function.
    pub fn execute_effect_static(lua: &Lua, data_arc: Arc<Mutex<DuelData>>, effect_id: EffectId, _code: u32, event_cards: Option<Group>, reason_effect: Option<EffectId>, reason_player: u8, trigger_player: u8) -> mlua::Result<()> {
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
            let args = Duel::get_lua_args(lua, effect_id, trigger_player, &event_cards, reason_player, reason_effect)?;
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
    BattleStart,
    BattleStep,
    Damage,
    DamageCal,
    Battle,
    Main2,
    End,
    GameOver,
}

impl Duel {
    /// Process the duel state machine for one cycle: returns true to continue, false to stop.
    pub fn process(&mut self) -> bool {
        // Priority: if there is a chain, resolve it before doing anything else
        {
            let data = self.data.lock().unwrap();
            if data.chain.links.len() > 0 {
                drop(data);
                self.resolve_chain();
                return true;
            }
        }
        // No chain to resolve: handle processor state
        let state_copy = { let data = self.data.lock().unwrap(); data.state };
        match state_copy {
            ProcessorState::Start => self.process_start(),
            ProcessorState::TurnChange => self.process_turn_change(),
            ProcessorState::Draw => self.process_draw_phase(),
            ProcessorState::Standby => {
                println!("Standby Phase");
                // For now just move to Main1
                let mut data = self.data.lock().unwrap();
                data.state = ProcessorState::Main1;
                true
            }
            ProcessorState::Main1 => self.process_main_phase(crate::core::enums::Phase::MAIN1),
            ProcessorState::BattleStart => {
                println!("Battle Start Phase");
                false
            }
            ProcessorState::BattleStep => {
                println!("Battle Step Phase");
                false
            }
            ProcessorState::Damage => {
                println!("Damage Phase");
                false
            }
            ProcessorState::DamageCal => {
                println!("Damage Calculation Phase");
                false
            }
            ProcessorState::Battle => {
                println!("Battle Phase");
                false
            }
            ProcessorState::Main2 => self.process_main_phase(crate::core::enums::Phase::MAIN2),
            ProcessorState::End => {
                println!("End Phase");
                // For now go to TurnChange
                let mut data = self.data.lock().unwrap();
                data.state = ProcessorState::TurnChange;
                true
            }
            ProcessorState::GameOver => {
                println!("Game Over");
                false
            }
        }
    }

    /// Create a card in the arena and return its CardId handle.
    pub fn create_card(&mut self, code: u32, owner: u8) -> CardId {
        let mut card = Card::new(code);
        // If a database entry exists, populate stats
        let db_arc = {
            let d = self.data.lock().unwrap();
            d.database.clone()
        };
        if let Ok(mut dbg) = db_arc.lock() {
            if let Ok(Some(cdata)) = dbg.query_card(code) {
                    card.original_stats.level = cdata.level;
                    card.original_stats.attack = cdata.attack;
                    card.original_stats.base_attack = cdata.attack;
                    card.original_stats.base_defense = cdata.defense;
                    card.original_stats.defense = cdata.defense;
                    // other fields
                    // We don't map type_/attribute/race enums directly here for simplicity
            }
        }
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

    /// Shuffle the specified player's deck using Fisher-Yates algorithm.
    pub fn shuffle_deck(&mut self, player: u8) {
        let mut data = self.data.lock().unwrap();
        data.shuffle_deck(player);
    }

    pub fn get_next_integer(rng: &mut Mt19937, min: u32, max: u32) -> u32 {
        let range = max - min + 1;
        let neg_range = (u32::MAX as u64).wrapping_add(1).wrapping_sub(range as u64);
        let bound = (neg_range % range as u64) as u32;
        let mut x = rng.gen_u32();
        while x < bound {
            x = rng.gen_u32();
        }
        min + (x % range)
    }

    /// Draw `count` cards from player's deck to their hand (append to hand).
    pub fn draw(&mut self, player: u8, count: u32) {
        let mut data = self.data.lock().unwrap();
        data.draw(player, count);
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
        // Step the processor and assert expected state transitions
        assert!(d.process(), "Start->TurnChange should continue");
        {
            let data = d.data.lock().unwrap();
            assert_eq!(data.state, ProcessorState::TurnChange);
        }
        assert!(d.process(), "TurnChange->Draw should continue");
        {
            let data = d.data.lock().unwrap();
            assert_eq!(data.state, ProcessorState::Draw);
        }
        assert!(d.process(), "Draw->Main1 should continue");
        {
            let data = d.data.lock().unwrap();
            assert_eq!(data.state, ProcessorState::Main1);
        }
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
    fn test_create_card_from_db() {
        // Build an in-memory DB and insert a card record
        let db = crate::core::database::Database::open_in_memory().expect("open in memory");
        db.conn.execute(
            "CREATE TABLE datas (id INTEGER, alias INTEGER, setcode INTEGER, type INTEGER, level INTEGER, attribute INTEGER, race INTEGER, atk INTEGER, def INTEGER);",
            rusqlite::params![]
        ).unwrap();
        db.conn.execute(
            "INSERT INTO datas (id, alias, setcode, type, level, attribute, race, atk, def) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![555i64, 0i64, 0i64, 1i64, 4i64, 1i64, 1i64, 2000i64, 1500i64]).unwrap();

        let db_arc = Arc::new(Mutex::new(db));
        let mut duel = Duel::new_with_db(42, db_arc);
        let id = duel.create_card(555, 0);
        let data = duel.data.lock().unwrap();
        let card = data.get_card(id).unwrap();
        assert_eq!(card.original_stats.attack, 2000);
        assert_eq!(card.original_stats.defense, 1500);
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

    #[test]
    fn test_replay_state_initialization() {
        use crate::core::replay::{Replay, ReplayHeader, DeckArray};
        
        // Create a mock replay with specific seed and decks
        let header = ReplayHeader {
            id: crate::core::replay::REPLAY_ID_YRP1,
            version: 0x12d0,
            flag: 0,
            seed: 12345, // Specific seed for deterministic testing
            datasize: 0,
            start_time: 0,
            props: [0u8; 8],
        };
        
        let decks = vec![
            DeckArray {
                main: vec![1001, 1002, 1003], // Player 0 main deck
                extra: vec![2001, 2002],       // Player 0 extra deck
            },
            DeckArray {
                main: vec![3001, 3002],        // Player 1 main deck
                extra: vec![4001],             // Player 1 extra deck
            },
        ];
        
        let replay = Replay {
            header,
            players: vec!["Player0".to_string(), "Player1".to_string()],
            params: crate::core::replay::DuelParameters::default(),
            decks,
            script_name: None,
            data: Vec::new(),
            actions: Vec::new(),
            packet_data: Vec::new(),
            decompressed_ok: true,
        };
        
        // Create duel and load replay
        let mut duel = Duel::new(999); // Initial seed different from replay
        duel.load_replay(replay);
        
        // Verify state after replay loading
        let mut data = duel.data.lock().unwrap();
        
        // Check RNG was initialized with replay seed by comparing with a fresh instance
        let mut test_rng = Mt19937::new(12345);
        let expected_first = test_rng.gen_u32();
        
        // Test the RNG with the mutable reference
        let actual_first = data.random.gen_u32();
        assert_eq!(actual_first, expected_first, "RNG should produce deterministic output based on replay seed");
        
        // Check processor state was reset
        assert_eq!(data.state, ProcessorState::Start, "Processor state should be reset to Start");
        
        // Check turn counter was reset
        assert_eq!(data.turn, 0, "Turn counter should be reset to 0");
        
        // Verify deck contents
        assert_eq!(data.field.deck[0].len(), 3, "Player 0 should have 3 main deck cards");
        assert_eq!(data.field.deck[1].len(), 2, "Player 1 should have 2 main deck cards");
        
        // Verify extra deck contents
        assert_eq!(data.field.extra[0].len(), 2, "Player 0 should have 2 extra deck cards");
        assert_eq!(data.field.extra[1].len(), 1, "Player 1 should have 1 extra deck card");
        
        // Verify card codes in decks
        let check_card_code = |player: usize, location: Location, expected_codes: &[u32]| {
            let cards = match location {
                Location::DECK => &data.field.deck[player],
                Location::EXTRA => &data.field.extra[player],
                _ => panic!("Unexpected location"),
            };
            
            for (i, &expected_code) in expected_codes.iter().enumerate() {
                if i < cards.len() {
                    let card_id = cards[i];
                    if let Some(card) = data.get_card(card_id) {
                        assert_eq!(card.code, expected_code, "Card {} in {:?} should have code {}", i, location, expected_code);
                        assert_eq!(card.owner, player as u8, "Card should belong to player {}", player);
                        assert!(card.location.contains(location), "Card should be in {:?}", location);
                    } else {
                        panic!("Card with id {} not found", card_id.0);
                    }
                }
            }
        };
        
        check_card_code(0, Location::DECK, &[1001, 1002, 1003]);
        check_card_code(0, Location::EXTRA, &[2001, 2002]);
        check_card_code(1, Location::DECK, &[3001, 3002]);
        check_card_code(1, Location::EXTRA, &[4001]);
    }

    #[test]
    fn test_simulation_initial_hand() {
        use std::path::PathBuf;
        use crate::core::replay::Replay;

        // Candidate paths to find the test replay file relative to the crate working dir
        let candidates = vec![
            PathBuf::from("../test/replay/2024-10-01 22-04-42(1).yrp"),
            PathBuf::from("../../test/replay/2024-10-01 22-04-42(1).yrp"),
        ];
        let mut found: Option<PathBuf> = None;
        for p in candidates.iter() {
            if p.exists() {
                found = Some(p.clone());
                break;
            }
        }
        let replay_path = found.expect("Could not locate test replay file; adjust path as necessary");
        println!("Using replay file: {:?}", replay_path);

        let r = Replay::open(&replay_path).expect("Failed to parse replay file");
        // Create duel and load replay
        let mut duel = Duel::new(42);
        duel.load_replay(r);

        // Run the processing loop until it waits on input at Main1
        let mut steps = 0;
        while duel.process() && steps < 100 {
            steps += 1;
        }

        let data = duel.data.lock().unwrap();
        // Turn player should have drawn one extra card (6), other player has 5
        let tp = data.turn_player as usize;
        let other = 1 - tp;
        println!("tp={} hand sizes: {} {}", tp, data.field.hand[tp].len(), data.field.hand[other].len());
        assert!(data.field.hand[tp].len() == 6, "Turn player should have 6 cards after initial draw");
        assert!(data.field.hand[other].len() == 5, "Non-turn player should have 5 cards after initial draw");
        assert_eq!(data.state, ProcessorState::Main1);
        assert_eq!(data.lp[0], 8000);
        assert_eq!(data.lp[1], 8000);
    }

    #[test]
    fn test_lua_args_integrity() {
        let mut duel = Duel::new(42);
        let card_id = duel.create_card(12345, 0);
        duel.move_card(card_id, 0, Location::HAND, 0);

        // Register an effect which uses tp and e in condition and operation
        let script = format!(r#"
            local c = Card({})
            local e = Effect.CreateEffect(c)
            e:SetCode(0x1000)
            e:SetCondition(function(e, tp, eg, ep, ev, re, r, rp)
                if tp ~= 0 then return false end
                if e == nil then return false end
                return true
            end)
            e:SetOperation(function(e, tp, eg, ep, ev, re, r, rp)
                local h = e:GetHandler()
                if h:GetCode() == 12345 then Duel.SendtoGrave(h, REASON_EFFECT) end
            end)
            c:RegisterEffect(e)
            return nil
        "#, card_id.0);
        let result: mlua::Result<()> = duel.lua.load(&script).exec();
        assert!(result.is_ok(), "Lua script to register effect should run");

        // Summon triggers the chain
        let res: mlua::Result<u32> = duel.lua.load(format!(r#"
            local c = Card({})
            return Duel.Summon(0, c, false, nil)
        "#, card_id.0)).eval();
        assert!(res.is_ok(), "Duel.Summon should work");

        // Chain should have item; resolve to execute and move card to grave
        {
            let data = duel.data.lock().unwrap();
            assert_eq!(data.chain.links.len(), 1, "Chain should have one link");
        }
        duel.resolve_chain();
        let data = duel.data.lock().unwrap();
        let card = data.get_card(card_id).expect("card exists");
        assert!(card.location.contains(Location::GRAVE), "Card should be moved to grave by operation");
    }
}
