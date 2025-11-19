use crate::core::types::CardId;
use mlua::{UserData, UserDataMethods};

/// Basic Effect structure for now
#[derive(Debug, Clone)]
pub struct Effect {
    pub id: u32,
    pub owner: CardId,
    pub description: u32,
    pub code: u32,
    pub flag: u32,
}

impl Effect {
    pub fn new(id: u32, owner: CardId, description: u32, code: u32, flag: u32) -> Self {
        Effect { id, owner, description, code, flag }
    }

    /// Create a new effect (static constructor for Lua)
    pub fn create_effect(_card: Option<CardId>) -> Self {
        Effect {
            id: 0,
            owner: CardId::new(0),
            description: 0,
            code: 0,
            flag: 0,
        }
    }
}

impl UserData for Effect {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        // Setter methods - all stubs for now
        methods.add_method_mut("SetDescription", |_, self_, desc: u32| {
            self_.description = desc;
            Ok(())
        });
        
        methods.add_method_mut("SetCode", |_, self_, code: u32| {
            self_.code = code;
            Ok(())
        });
        
        methods.add_method_mut("SetRange", |_, _self, _range: u32| {
            // Stub - store range somewhere if needed
            Ok(())
        });
        
        methods.add_method_mut("SetType", |_, _self, _effect_type: u32| {
            // Stub - store type somewhere if needed
            Ok(())
        });
        
        methods.add_method_mut("SetProperty", |_, _self, _property: u32| {
            // Stub - store property somewhere if needed
            Ok(())
        });
        
        methods.add_method_mut("SetCondition", |_, _self, _condition: mlua::Function| {
            // Stub - store condition function if needed
            Ok(())
        });
        
        methods.add_method_mut("SetCost", |_, _self, _cost: mlua::Function| {
            // Stub - store cost function if needed
            Ok(())
        });
        
        methods.add_method_mut("SetTarget", |_, _self, _target: mlua::Function| {
            // Stub - store target function if needed
            Ok(())
        });
        
        methods.add_method_mut("SetOperation", |_, _self, _operation: mlua::Function| {
            // Stub - store operation function if needed
            Ok(())
        });
        
        methods.add_method_mut("SetValue", |_, _self, _value: mlua::Value| {
            // Stub - store value if needed
            Ok(())
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::CardId;

    #[test]
    fn effect_constructs() {
        let e = Effect::new(1, CardId::new(0), 10, 100, 0);
        assert_eq!(e.id, 1);
        assert_eq!(e.owner, CardId::new(0));
    }
}
