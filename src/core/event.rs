use crate::core::types::EffectId;
use crate::core::group::Group;

/// Event structure used to notify the engine and trigger effects.
#[derive(Debug, Clone)]
pub struct Event {
    pub code: u32,
    pub reason_effect: Option<EffectId>,
    pub reason_player: u8,
    pub event_cards: Option<Group>,
}

impl Event {
    pub fn new(code: u32, reason_effect: Option<EffectId>, reason_player: u8, event_cards: Option<Group>) -> Self {
        Event { code, reason_effect, reason_player, event_cards }
    }
}
