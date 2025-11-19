use crate::core::enums::*;
use mlua::{UserData, UserDataMethods};
use crate::core::types::{CardId, EffectId};
use std::sync::{Arc, Mutex};

/// StatBlock stores a card's original/current mutable attributes
// Keep traits minimal to avoid relying on derived traits from bitflags
#[derive(Clone)]
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
#[derive(Clone)]
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
    // Associated effects
    pub effects: Vec<EffectId>,
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
            effects: vec![],
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
        methods.add_method_mut("RegisterEffect", |lua, self_, effect_ud: mlua::AnyUserData| {
            // Register the effect in the DuelData arena and attach to this card
            let data = lua.app_data_ref::<Arc<Mutex<crate::core::duel::DuelData>>>()
                .expect("DuelData not found in Lua app data");
            // Copy effect data from userdata and move into arena by making a fresh Effect instance.
            // Extract fields from userdata, then build a new Effect with cloned fields and newly created registry keys.
            if let Ok(e) = effect_ud.borrow::<crate::core::effect::Effect>() {
                // Recreate registry keys by extracting function and creating a new registry entry for it.
                let mut cond_key = None;
                if let Some(k) = &e.condition {
                    if let Ok(func) = lua.registry_value::<mlua::Function>(k) {
                        if let Ok(newk) = lua.create_registry_value(func) { cond_key = Some(newk); }
                    }
                }
                let mut cost_key = None;
                if let Some(k) = &e.cost {
                    if let Ok(func) = lua.registry_value::<mlua::Function>(k) {
                        if let Ok(newk) = lua.create_registry_value(func) { cost_key = Some(newk); }
                    }
                }
                let mut target_key = None;
                if let Some(k) = &e.target {
                    if let Ok(func) = lua.registry_value::<mlua::Function>(k) {
                        if let Ok(newk) = lua.create_registry_value(func) { target_key = Some(newk); }
                    }
                }
                let mut op_key = None;
                if let Some(k) = &e.operation {
                    if let Ok(func) = lua.registry_value::<mlua::Function>(k) {
                        if let Ok(newk) = lua.create_registry_value(func) { op_key = Some(newk); }
                    }
                }
                let new_effect = crate::core::effect::Effect {
                    id: 0,
                    owner: *self_,
                    description: e.description,
                    code: e.code,
                    type_: e.type_,
                    range: e.range,
                    flag: e.flag,
                    condition: cond_key,
                    cost: cost_key,
                    target: target_key,
                    operation: op_key,
                };
                let mut data_guard = data.lock().unwrap();
                let _eid = data_guard.register_effect(new_effect, Some(*self_));
            }
            Ok(())
        });
        
        // Method: c:GetCode() - returns card code
        methods.add_method("GetCode", |lua, self_, ()| {
            // Get the actual card code from the duel data
            let data = lua.app_data_ref::<Arc<Mutex<crate::core::duel::DuelData>>>()
                .expect("DuelData not found in Lua app data");
            let data_guard = data.lock().unwrap();
            
            if let Some(card) = data_guard.get_card(*self_) {
                Ok(card.code)
            } else {
                Ok(0) // Return 0 if card not found
            }
        });
        
        // Method: c:GetControler() - returns controller
        methods.add_method("GetControler", |lua, self_, ()| {
            // Get the actual controller from the duel data
            let data = lua.app_data_ref::<Arc<Mutex<crate::core::duel::DuelData>>>()
                .expect("DuelData not found in Lua app data");
            let data_guard = data.lock().unwrap();
            
            if let Some(card) = data_guard.get_card(*self_) {
                Ok(card.controller as u32)
            } else {
                Ok(0) // Return 0 if card not found
            }
        });
        
        // Method: c:GetLocation() - returns location
        methods.add_method("GetLocation", |lua, self_, ()| {
            // Get the actual location from the duel data
            let data = lua.app_data_ref::<Arc<Mutex<crate::core::duel::DuelData>>>()
                .expect("DuelData not found in Lua app data");
            let data_guard = data.lock().unwrap();
            
            if let Some(card) = data_guard.get_card(*self_) {
                Ok(card.location.bits() as u32)
            } else {
                Ok(0) // Return 0 if card not found
            }
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
