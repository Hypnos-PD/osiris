//! Processor Unit Queue implementation
//! Based on ygopro's core.units architecture



/// Result of processing a unit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessResult {
    /// Continue processing next unit
    Continue,
    /// Waiting for input or external event
    Waiting,
    /// Processing ended (no more units)
    End,
}

/// Types of processor units
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessorType {
    /// Main turn processing
    Turn,
    /// Phase event processing
    PhaseEvent,
    /// Point event processing
    PointEvent,
    /// Quick effect processing
    QuickEffect,
    /// Idle command processing
    IdleCommand,
    /// Card selection processing
    SelectCard,
    /// Chain selection processing
    SelectChain,
    /// Battle processing
    Battle,
    /// Damage calculation
    DamageCalc,
    /// Summon processing
    Summon,
    /// Position change
    Position,
    /// Special summon
    SpecialSummon,
    /// Normal summon
    NormalSummon,
    /// Set monster
    SetMonster,
    /// Set spell/trap
    SetSpellTrap,
    /// Activate effect
    ActivateEffect,
    /// Resolve chain
    ResolveChain,
    /// Solve chain (process chain resolution)
    SolveChain,
}

/// A processor unit representing a discrete processing step
#[derive(Debug, Clone)]
pub struct ProcessorUnit {
    /// Type of processing unit
    pub type_: ProcessorType,
    /// Current step within this unit
    pub step: u32,
    /// Generic argument 1
    pub arg1: u32,
    /// Generic argument 2
    pub arg2: u32,
}

impl ProcessorUnit {
    /// Create a new processor unit
    pub fn new(type_: ProcessorType, step: u32, arg1: u32, arg2: u32) -> Self {
        Self {
            type_,
            step,
            arg1,
            arg2,
        }
    }

    /// Create a turn processor unit
    pub fn turn(step: u32) -> Self {
        Self::new(ProcessorType::Turn, step, 0, 0)
    }

    /// Create a phase event processor unit
    pub fn phase_event(step: u32, phase: u32) -> Self {
        Self::new(ProcessorType::PhaseEvent, step, phase, 0)
    }

    /// Create a solve chain processor unit
    pub fn solve_chain(step: u32) -> Self {
        Self::new(ProcessorType::SolveChain, step, 0, 0)
    }
}