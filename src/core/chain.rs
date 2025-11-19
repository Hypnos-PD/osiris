// use crate::core::effect::Effect; // unused now
use crate::core::types::EffectId;
use crate::core::group::Group;

pub struct ChainLink {
    pub effect_id: EffectId,
    pub trigger_player: u8,
    pub target_cards: Option<Group>,
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
        let l1 = ChainLink { effect_id: EffectId::new(1), trigger_player: 0, target_cards: None };
        let l2 = ChainLink { effect_id: EffectId::new(2), trigger_player: 1, target_cards: None };
        c.add(l1);
        c.add(l2);
        let r = c.pop().expect("link");
        assert_eq!(r.effect_id.0, 2);
        let r2 = c.pop().expect("link2");
        assert_eq!(r2.effect_id.0, 1);
    }
}
