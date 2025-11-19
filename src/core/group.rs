use mlua::{UserData, UserDataMethods, MetaMethod};
use crate::core::types::CardId;

/// Represents a collection of unique Card IDs
#[derive(Debug, Clone)]
pub struct Group(pub std::collections::HashSet<CardId>);

impl Group {
    /// Creates a new empty Group
    pub fn new() -> Self {
        Group(std::collections::HashSet::new())
    }
}

impl UserData for Group {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        // Method: g:GetCount() - returns the number of cards in the group
        methods.add_method("GetCount", |_, self_, ()| {
            Ok(self_.0.len() as u32)
        });
        
        // Meta method: #g - returns the number of cards in the group
        methods.add_meta_method(MetaMethod::Len, |_, self_, ()| {
            Ok(self_.0.len() as u32)
        });
        
        // Method: g:AddCard(card) - stub for now, accepts CardId for testing
        // In reality, this should accept a Card UserData, but we'll use CardId for initial testing
        methods.add_method_mut("AddCard", |_, self_, card_id: CardId| {
            self_.0.insert(card_id);
            Ok(())
        });
    }
}