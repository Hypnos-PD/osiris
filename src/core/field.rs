use crate::core::types::CardId;

/// Field stores lists of CardId for each player (0/1) and fixed-size zones.
pub struct Field {
    pub deck: [Vec<CardId>; 2],
    pub hand: [Vec<CardId>; 2],
    pub grave: [Vec<CardId>; 2],
    pub remove: [Vec<CardId>; 2],
    pub extra: [Vec<CardId>; 2],
    pub mzone: [[Option<CardId>; 7]; 2],
    pub szone: [[Option<CardId>; 8]; 2],
}

impl Field {
    pub fn new() -> Self {
        Field {
            deck: [Vec::new(), Vec::new()],
            hand: [Vec::new(), Vec::new()],
            grave: [Vec::new(), Vec::new()],
            remove: [Vec::new(), Vec::new()],
            extra: [Vec::new(), Vec::new()],
            mzone: [[None; 7], [None; 7]],
            szone: [[None; 8], [None; 8]],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::CardId;

    #[test]
    fn field_new_empty() {
        let f = Field::new();
        assert_eq!(f.deck[0].len(), 0);
        assert_eq!(f.hand[1].len(), 0);
        assert!(f.mzone[0][0].is_none());
        assert!(f.szone[1][7].is_none());
        // add a card to deck 0
        let mut f2 = f;
        f2.deck[0].push(CardId::new(1));
        assert_eq!(f2.deck[0].len(), 1);
    }
}
