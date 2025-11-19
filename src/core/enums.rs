use bitflags::bitflags;

// Card type flags (TYPE_* in C++)
bitflags! {
    pub struct CardType: u32 {
        const MONSTER = 0x1; // TYPE_MONSTER
        const SPELL = 0x2; // TYPE_SPELL
        const TRAP = 0x4; // TYPE_TRAP
        const NORMAL = 0x10; // TYPE_NORMAL
        const EFFECT = 0x20; // TYPE_EFFECT
        const FUSION = 0x40; // TYPE_FUSION
        const RITUAL = 0x80; // TYPE_RITUAL
        const TRAPMONSTER = 0x100; // TYPE_TRAPMONSTER
        const SPIRIT = 0x200; // TYPE_SPIRIT
        const UNION = 0x400; // TYPE_UNION
        const DUAL = 0x800; // TYPE_DUAL
        const TUNER = 0x1000; // TYPE_TUNER
        const SYNCHRO = 0x2000; // TYPE_SYNCHRO
        const TOKEN = 0x4000; // TYPE_TOKEN
        const QUICKPLAY = 0x10000; // TYPE_QUICKPLAY
        const CONTINUOUS = 0x20000; // TYPE_CONTINUOUS
        const EQUIP = 0x40000; // TYPE_EQUIP
        const FIELD = 0x80000; // TYPE_FIELD
        const COUNTER = 0x100000; // TYPE_COUNTER
        const FLIP = 0x200000; // TYPE_FLIP
        const TOON = 0x400000; // TYPE_TOON
        const XYZ = 0x800000; // TYPE_XYZ
        const PENDULUM = 0x1000000; // TYPE_PENDULUM
        const SPSUMMON = 0x2000000; // TYPE_SPSUMMON
        const LINK = 0x4000000; // TYPE_LINK
    }
}

// Card attribute flags (ATTRIBUTE_* in C++)
bitflags! {
    pub struct CardAttribute: u32 {
        const EARTH = 0x1; // ATTRIBUTE_EARTH
        const WATER = 0x2; // ATTRIBUTE_WATER
        const FIRE = 0x4; // ATTRIBUTE_FIRE
        const WIND = 0x8; // ATTRIBUTE_WIND
        const LIGHT = 0x10; // ATTRIBUTE_LIGHT
        const DARK = 0x20; // ATTRIBUTE_DARK
        const DEVINE = 0x40; // ATTRIBUTE_DEVINE
    }
}

// Card race flags (RACE_* in C++)
bitflags! {
    pub struct CardRace: u32 {
        const WARRIOR = 0x1; // RACE_WARRIOR
        const SPELLCASTER = 0x2; // RACE_SPELLCASTER
        const FAIRY = 0x4; // RACE_FAIRY
        const FIEND = 0x8; // RACE_FIEND
        const ZOMBIE = 0x10; // RACE_ZOMBIE
        const MACHINE = 0x20; // RACE_MACHINE
        const AQUA = 0x40; // RACE_AQUA
        const PYRO = 0x80; // RACE_PYRO
        const ROCK = 0x100; // RACE_ROCK
        const WINDBEAST = 0x200; // RACE_WINDBEAST
        const PLANT = 0x400; // RACE_PLANT
        const INSECT = 0x800; // RACE_INSECT
        const THUNDER = 0x1000; // RACE_THUNDER
        const DRAGON = 0x2000; // RACE_DRAGON
        const BEAST = 0x4000; // RACE_BEAST
        const BEASTWARRIOR = 0x8000; // RACE_BEASTWARRIOR
        const DINOSAUR = 0x10000; // RACE_DINOSAUR
        const FISH = 0x20000; // RACE_FISH
        const SEASERPENT = 0x40000; // RACE_SEASERPENT
        const REPTILE = 0x80000; // RACE_REPTILE
        const PSYCHO = 0x100000; // RACE_PSYCHO
        const DEVINE = 0x200000; // RACE_DEVINE
        const CREATORGOD = 0x400000; // RACE_CREATORGOD
        const WYRM = 0x800000; // RACE_WYRM
        const CYBERSE = 0x1000000; // RACE_CYBERSE
        const ILLUSION = 0x2000000; // RACE_ILLUSION
    }
}

// Card position flags (POS_* in C++)
bitflags! {
    pub struct CardPosition: u32 {
        const FACEUP_ATTACK = 0x1; // POS_FACEUP_ATTACK
        const FACEDOWN_ATTACK = 0x2; // POS_FACEDOWN_ATTACK
        const FACEUP_DEFENSE = 0x4; // POS_FACEUP_DEFENSE
        const FACEDOWN_DEFENSE = 0x8; // POS_FACEDOWN_DEFENSE
        // Common combinations
        const FACEUP = 0x5; // POS_FACEUP (FACEUP_ATTACK | FACEUP_DEFENSE)
        const FACEDOWN = 0xA; // POS_FACEDOWN (FACEDOWN_ATTACK | FACEDOWN_DEFENSE)
        const ATTACK = 0x3; // POS_ATTACK (FACEUP_ATTACK | FACEDOWN_ATTACK)
        const DEFENSE = 0xC; // POS_DEFENSE (FACEUP_DEFENSE | FACEDOWN_DEFENSE)
    }
}

// Location flags (LOCATION_* in C++)
bitflags! {
    pub struct Location: u32 {
        const DECK = 0x1; // LOCATION_DECK
        const HAND = 0x2; // LOCATION_HAND
        const MZONE = 0x4; // LOCATION_MZONE
        const SZONE = 0x8; // LOCATION_SZONE
        const GRAVE = 0x10; // LOCATION_GRAVE
        const REMOVED = 0x20; // LOCATION_REMOVED
        const EXTRA = 0x40; // LOCATION_EXTRA
        const OVERLAY = 0x80; // LOCATION_OVERLAY
        const ONFIELD = 0xC; // LOCATION_ONFIELD (MZONE | SZONE)
        const FZONE = 0x100; // LOCATION_FZONE
        const PZONE = 0x200; // LOCATION_PZONE
        const DECKBOT = 0x10001; // LOCATION_DECKBOT (65537)
        const DECKSHF = 0x20001; // LOCATION_DECKSHF (131073)
    }
}

// Phase flags (PHASE_* in C++)
bitflags! {
    pub struct Phase: u32 {
        const DRAW = 0x1; // PHASE_DRAW
        const STANDBY = 0x2; // PHASE_STANDBY
        const MAIN1 = 0x4; // PHASE_MAIN1
        const BATTLE_START = 0x8; // PHASE_BATTLE_START
        const BATTLE_STEP = 0x10; // PHASE_BATTLE_STEP
        const DAMAGE = 0x20; // PHASE_DAMAGE
        const DAMAGE_CAL = 0x40; // PHASE_DAMAGE_CAL
        const BATTLE = 0x80; // PHASE_BATTLE
        const MAIN2 = 0x100; // PHASE_MAIN2
        const END = 0x200; // PHASE_END
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn sanity_check_types() {
        // basic sanity checks to ensure sizes & basic flags are OK
        assert_eq!(mem::size_of::<CardType>(), mem::size_of::<u32>());
        assert_eq!(mem::size_of::<CardAttribute>(), mem::size_of::<u32>());
        assert_eq!(mem::size_of::<CardRace>(), mem::size_of::<u32>());
    }
}
