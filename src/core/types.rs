use mlua::{FromLua, Lua, Value};

/// Core typed IDs and handles to avoid mixing up card database code with internal handle index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CardId(pub u32);

impl CardId {
    pub fn new(idx: u32) -> Self {
        CardId(idx)
    }
    pub fn as_u32(self) -> u32 { self.0 }
}

impl Default for CardId {
    fn default() -> Self { CardId(0) }
}

/// An index into DuelData::effects Sandboxed by the Duel so script-managed effects are stored somewhere
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectId(pub u32);

impl EffectId {
    pub fn new(idx: u32) -> Self { EffectId(idx) }
    pub fn as_u32(self) -> u32 { self.0 }
}

impl Default for EffectId { fn default() -> Self { EffectId(0) } }

impl<'lua> FromLua<'lua> for CardId {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> mlua::Result<Self> {
        match value {
            Value::Integer(i) => Ok(CardId(i as u32)),
            Value::Number(n) => Ok(CardId(n as u32)),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "CardId",
                message: Some("Expected integer or number".to_string()),
            }),
        }
    }
}
