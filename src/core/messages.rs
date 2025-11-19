// Message protocol definitions — derived from YGOPRO common.h (MSG_* constants)
#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MsgType {
    Unknown(u8),
    Retry = 1,
    Hint = 2,
    Waiting = 3,
    Start = 4,
    Win = 5,
    UpdateData = 6,
    UpdateCard = 7,
    RequestDeck = 8,
    SelectBattleCmd = 10,
    SelectIdleCmd = 11,
    SelectEffectYN = 12,
    SelectYesNo = 13,
    SelectOption = 14,
    SelectCard = 15,
    SelectChain = 16,
    SelectPlace = 18,
    SelectPosition = 19,
    SelectTribute = 20,
    SortChain = 21,
    SelectCounter = 22,
    SelectSum = 23,
    SelectDisField = 24,
    SortCard = 25,
    SelectUnselectCard = 26,
    ConfirmDeckTop = 30,
    ConfirmCards = 31,
    ShuffleDeck = 32,
    ShuffleHand = 33,
    RefreshDeck = 34,
    SwapGraveDeck = 35,
    ShuffleSetCard = 36,
    ReverseDeck = 37,
    DeckTop = 38,
    NewTurn = 40,
    NewPhase = 41,
    ConfirmExtraTop = 42,
    Move = 50,
    PosChange = 53,
    Set = 54,
    Swap = 55,
    FieldDisabled = 56,
    Summoning = 60,
    Summoned = 61,
    SPSummoning = 62,
    SPSummoned = 63,
    FlipSummoning = 64,
    FlipSummoned = 65,
    Chaining = 70,
    Chained = 71,
    ChainSolving = 72,
    ChainSolved = 73,
    ChainEnd = 74,
    ChainNegated = 75,
    ChainDisabled = 76,
    CardSelected = 80,
    RandomSelected = 81,
    BecomeTarget = 83,
    Draw = 90,
    Damage = 91,
    Recover = 92,
    Equip = 93,
    LpUpdate = 94,
    Unequip = 95,
    CardTarget = 96,
    CancelTarget = 97,
    PayLpCost = 100,
    AddCounter = 101,
    RemoveCounter = 102,
    Attack = 110,
    Battle = 111,
    AttackDisabled = 112,
    DamageStepStart = 113,
    DamageStepEnd = 114,
    MissedEffect = 120,
    BeChainTarget = 121,
    CreateRelation = 122,
    ReleaseRelation = 123,
    TossCoin = 130,
    TossDice = 131,
    RockPaperScissors = 132,
    HandRes = 133,
    AnnounceRace = 140,
    AnnounceAttrib = 141,
    AnnounceCard = 142,
    AnnounceNumber = 143,
    CardHint = 160,
    TagSwap = 161,
    ReloadField = 162,
    AiName = 163,
    ShowHint = 164,
    PlayerHint = 165,
    MatchKill = 170,
    CustomMsg = 180,
}

impl From<u8> for MsgType {
    fn from(v: u8) -> Self {
        match v {
            1 => MsgType::Retry,
            2 => MsgType::Hint,
            3 => MsgType::Waiting,
            4 => MsgType::Start,
            5 => MsgType::Win,
            6 => MsgType::UpdateData,
            7 => MsgType::UpdateCard,
            8 => MsgType::RequestDeck,
            10 => MsgType::SelectBattleCmd,
            11 => MsgType::SelectIdleCmd,
            12 => MsgType::SelectEffectYN,
            13 => MsgType::SelectYesNo,
            14 => MsgType::SelectOption,
            15 => MsgType::SelectCard,
            16 => MsgType::SelectChain,
            18 => MsgType::SelectPlace,
            19 => MsgType::SelectPosition,
            20 => MsgType::SelectTribute,
            21 => MsgType::SortChain,
            22 => MsgType::SelectCounter,
            23 => MsgType::SelectSum,
            24 => MsgType::SelectDisField,
            25 => MsgType::SortCard,
            26 => MsgType::SelectUnselectCard,
            30 => MsgType::ConfirmDeckTop,
            31 => MsgType::ConfirmCards,
            32 => MsgType::ShuffleDeck,
            33 => MsgType::ShuffleHand,
            34 => MsgType::RefreshDeck,
            35 => MsgType::SwapGraveDeck,
            36 => MsgType::ShuffleSetCard,
            37 => MsgType::ReverseDeck,
            38 => MsgType::DeckTop,
            40 => MsgType::NewTurn,
            41 => MsgType::NewPhase,
            42 => MsgType::ConfirmExtraTop,
            50 => MsgType::Move,
            53 => MsgType::PosChange,
            54 => MsgType::Set,
            55 => MsgType::Swap,
            56 => MsgType::FieldDisabled,
            60 => MsgType::Summoning,
            61 => MsgType::Summoned,
            62 => MsgType::SPSummoning,
            63 => MsgType::SPSummoned,
            64 => MsgType::FlipSummoning,
            65 => MsgType::FlipSummoned,
            70 => MsgType::Chaining,
            71 => MsgType::Chained,
            72 => MsgType::ChainSolving,
            73 => MsgType::ChainSolved,
            74 => MsgType::ChainEnd,
            75 => MsgType::ChainNegated,
            76 => MsgType::ChainDisabled,
            80 => MsgType::CardSelected,
            81 => MsgType::RandomSelected,
            83 => MsgType::BecomeTarget,
            90 => MsgType::Draw,
            91 => MsgType::Damage,
            92 => MsgType::Recover,
            93 => MsgType::Equip,
            94 => MsgType::LpUpdate,
            95 => MsgType::Unequip,
            96 => MsgType::CardTarget,
            97 => MsgType::CancelTarget,
            100 => MsgType::PayLpCost,
            101 => MsgType::AddCounter,
            102 => MsgType::RemoveCounter,
            110 => MsgType::Attack,
            111 => MsgType::Battle,
            112 => MsgType::AttackDisabled,
            113 => MsgType::DamageStepStart,
            114 => MsgType::DamageStepEnd,
            120 => MsgType::MissedEffect,
            121 => MsgType::BeChainTarget,
            122 => MsgType::CreateRelation,
            123 => MsgType::ReleaseRelation,
            130 => MsgType::TossCoin,
            131 => MsgType::TossDice,
            132 => MsgType::RockPaperScissors,
            133 => MsgType::HandRes,
            140 => MsgType::AnnounceRace,
            141 => MsgType::AnnounceAttrib,
            142 => MsgType::AnnounceCard,
            143 => MsgType::AnnounceNumber,
            160 => MsgType::CardHint,
            161 => MsgType::TagSwap,
            162 => MsgType::ReloadField,
            163 => MsgType::AiName,
            164 => MsgType::ShowHint,
            165 => MsgType::PlayerHint,
            170 => MsgType::MatchKill,
            180 => MsgType::CustomMsg,
            x => MsgType::Unknown(x),
        }
    }
}

/// Parse a packet (first byte is message id), return the MsgType and the payload slice
pub fn parse_packet(data: &[u8]) -> (MsgType, &[u8]) {
    if data.is_empty() { return (MsgType::Unknown(0), data); }
    let id = data[0];
    (MsgType::from(id), &data[1..])
}

// Payload parsers for some important message types
use std::io::Cursor;
use byteorder::{LittleEndian, ReadBytesExt};
/// Start message payload
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MsgStart {
    pub player_type: u8,
    pub lp: [u32; 2],
    pub deck_count: [u16; 2],
    pub extra_count: [u16; 2],
    pub hand_count: [u16; 2],
}

impl MsgStart {
    pub fn parse(payload: &[u8]) -> Option<MsgStart> {
        let mut cursor = Cursor::new(payload);
        let player_type = cursor.read_u8().ok()?;
        let lp0 = cursor.read_u32::<LittleEndian>().ok()?;
        let lp1 = cursor.read_u32::<LittleEndian>().ok()?;
        let d0 = cursor.read_u16::<LittleEndian>().ok()?;
        let d1 = cursor.read_u16::<LittleEndian>().ok()?;
        let e0 = cursor.read_u16::<LittleEndian>().ok()?;
        let e1 = cursor.read_u16::<LittleEndian>().ok()?;
        let h0 = cursor.read_u16::<LittleEndian>().ok()?;
        let h1 = cursor.read_u16::<LittleEndian>().ok()?;
        Some(MsgStart { player_type, lp: [lp0, lp1], deck_count: [d0, d1], extra_count: [e0, e1], hand_count: [h0, h1] })
    }
}

/// New turn payload
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MsgNewTurn { pub player: u8 }

impl MsgNewTurn { pub fn parse(payload: &[u8]) -> Option<MsgNewTurn> { Some(MsgNewTurn { player: Cursor::new(payload).read_u8().ok()? }) } }

/// Draw payload: player, count
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MsgDraw { pub player: u8, pub count: u8 }

impl MsgDraw {
    pub fn parse(payload: &[u8]) -> Option<MsgDraw> {
        let mut cursor = Cursor::new(payload);
        let player = cursor.read_u8().ok()?;
        let count = cursor.read_u8().ok()?;
        Some(MsgDraw { player, count })
    }
}

/// LP update: player, new LP (u32)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MsgLpUpdate { pub player: u8, pub lp: u32 }

impl MsgLpUpdate {
    pub fn parse(payload: &[u8]) -> Option<MsgLpUpdate> {
        let mut cursor = Cursor::new(payload);
        let player = cursor.read_u8().ok()?;
        let lp = cursor.read_u32::<LittleEndian>().ok()?;
        Some(MsgLpUpdate { player, lp })
    }
}

