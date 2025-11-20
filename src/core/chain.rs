// use crate::core::effect::Effect; // unused now
use crate::core::types::EffectId;
use crate::core::group::Group;

pub struct ChainLink {
    pub effect_id: EffectId,
    pub trigger_player: u8,
    pub check_player: u8,
    pub target_cards: Option<Group>,
    pub reason_effect: Option<EffectId>,
    pub reason_player: u8,
    pub evt_group: Option<Group>,
    pub evt_player: u8,
    pub evt_value: u32,
    pub evt_effect: Option<EffectId>,
    pub evt_reason: u32,
    pub evt_r_player: u8,
}

pub struct Chain {
    pub links: Vec<ChainLink>,
}

impl Chain {
    pub fn new() -> Self { Chain { links: Vec::new() } }

    pub fn add(&mut self, link: ChainLink) {
        self.links.push(link);
    }

    pub fn pop(&mut self) -> Option<ChainLink> {
        self.links.pop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::core::effect::Effect;
    // use crate::core::types::CardId;
    use crate::core::types::EffectId;
    // use crate::core::group::Group;

    #[test]
    fn chain_add_and_resolve() {
        let mut c = Chain::new();
        let l1 = ChainLink { 
            effect_id: EffectId::new(1), 
            trigger_player: 0, 
            check_player: 0,
            target_cards: None, 
            reason_effect: None, 
            reason_player: 0,
            evt_group: None,
            evt_player: 0,
            evt_value: 0,
            evt_effect: None,
            evt_reason: 0,
            evt_r_player: 0,
        };
        let l2 = ChainLink { 
            effect_id: EffectId::new(2), 
            trigger_player: 1, 
            check_player: 1,
            target_cards: None, 
            reason_effect: None, 
            reason_player: 1,
            evt_group: None,
            evt_player: 0,
            evt_value: 0,
            evt_effect: None,
            evt_reason: 0,
            evt_r_player: 0,
        };
        c.add(l1);
        c.add(l2);
        let r = c.pop().expect("link");
        assert_eq!(r.effect_id.0, 2);
        let r2 = c.pop().expect("link2");
        assert_eq!(r2.effect_id.0, 1);
    }
}
