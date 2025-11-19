use crate::core::effect::Effect;

pub struct Chain {
    pub links: Vec<Effect>,
}

impl Chain {
    pub fn new() -> Self { Chain { links: Vec::new() } }

    pub fn add_effect(&mut self, effect: Effect) {
        self.links.push(effect);
    }

    pub fn resolve(&mut self) -> Option<Effect> {
        self.links.pop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::effect::Effect;
    use crate::core::types::CardId;

    #[test]
    fn chain_add_and_resolve() {
        let mut c = Chain::new();
        let e1 = Effect::new(1, CardId::new(0), 1, 0, 0);
        let e2 = Effect::new(2, CardId::new(1), 2, 0, 0);
        c.add_effect(e1);
        c.add_effect(e2);
        let r = c.resolve().expect("effect");
        assert_eq!(r.id, 2);
        let r2 = c.resolve().expect("effect2");
        assert_eq!(r2.id, 1);
    }
}
