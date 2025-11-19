use crate::core::types::CardId;
use mlua::{UserData, UserDataMethods, RegistryKey, Function};

/// Basic Effect structure for now
#[derive(Debug)]
pub struct Effect {
    pub id: u32,
    pub owner: CardId,
    pub description: u32,
    pub code: u32,
    pub type_: u32,
    pub range: u32,
    pub flag: u32,
    pub condition: Option<RegistryKey>,
    pub cost: Option<RegistryKey>,
    pub target: Option<RegistryKey>,
    pub operation: Option<RegistryKey>,
}

impl Effect {
    pub fn new(id: u32, owner: CardId, description: u32, code: u32, type_: u32, range: u32, flag: u32) -> Self {
        Effect { id, owner, description, code, type_, range, flag, condition: None, cost: None, target: None, operation: None }
    }

    /// Create a new effect (static constructor for Lua)
    pub fn create_effect(_card: Option<CardId>) -> Self {
        Effect {
            id: 0,
            owner: CardId::new(0),
            description: 0,
            code: 0,
            type_: 0,
            range: 0,
            flag: 0,
            condition: None,
            cost: None,
            target: None,
            operation: None,
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
        
        methods.add_method_mut("SetRange", |_, self_, range: u32| {
            self_.range = range;
            Ok(())
        });
        
        methods.add_method_mut("SetType", |_, self_, effect_type: u32| {
            self_.type_ = effect_type;
            Ok(())
        });
        
        methods.add_method_mut("SetProperty", |_, _self, _property: u32| {
            // Stub - store property somewhere if needed
            Ok(())
        });
        
        methods.add_method_mut("SetCondition", |lua, self_, condition: Function| {
            match lua.create_registry_value(condition) {
                Ok(key) => { self_.condition = Some(key); Ok(()) },
                Err(e) => Err(e),
            }
        });
        
        methods.add_method_mut("SetCost", |lua, self_, cost: Function| {
            match lua.create_registry_value(cost) {
                Ok(key) => { self_.cost = Some(key); Ok(()) },
                Err(e) => Err(e),
            }
        });
        
        methods.add_method_mut("SetTarget", |lua, self_, target: Function| {
            match lua.create_registry_value(target) {
                Ok(key) => { self_.target = Some(key); Ok(()) },
                Err(e) => Err(e),
            }
        });
        
        methods.add_method_mut("SetOperation", |lua, self_, operation: Function| {
            match lua.create_registry_value(operation) {
                Ok(key) => { self_.operation = Some(key); Ok(()) },
                Err(e) => Err(e),
            }
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
        let e = Effect::new(1, CardId::new(0), 10, 100, 0, 0, 0);
        assert_eq!(e.id, 1);
        assert_eq!(e.owner, CardId::new(0));
    }
}
