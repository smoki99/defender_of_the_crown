#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UnitKind {
    Levy,
    Bowman,
    Knight,
    Siege,
}

pub const ALL_KINDS: [UnitKind; 4] = [
    UnitKind::Levy,
    UnitKind::Bowman,
    UnitKind::Knight,
    UnitKind::Siege,
];

impl UnitKind {
    pub fn name(self) -> &'static str {
        match self {
            UnitKind::Levy => "Levies",
            UnitKind::Bowman => "Bowmen",
            UnitKind::Knight => "Knights",
            UnitKind::Siege => "Catapults",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            UnitKind::Levy => "\u{1FA93}",   // axe
            UnitKind::Bowman => "\u{1F3F9}", // bow-ish banner
            UnitKind::Knight => "\u{1F40E}", // horse
            UnitKind::Siege => "\u{1F680}",  // trebuchet stand-in
        }
    }

    pub fn cost(self) -> u32 {
        match self {
            UnitKind::Levy => 10,
            UnitKind::Bowman => 20,
            UnitKind::Knight => 45,
            UnitKind::Siege => 60,
        }
    }

    pub fn hp(self) -> u32 {
        match self {
            UnitKind::Levy => 8,
            UnitKind::Bowman => 8,
            UnitKind::Knight => 18,
            UnitKind::Siege => 12,
        }
    }

    pub fn atk(self) -> u32 {
        match self {
            UnitKind::Levy => 3,
            UnitKind::Bowman => 5,
            UnitKind::Knight => 9,
            UnitKind::Siege => 7,
        }
    }

    pub fn index(self) -> usize {
        match self {
            UnitKind::Levy => 0,
            UnitKind::Bowman => 1,
            UnitKind::Knight => 2,
            UnitKind::Siege => 3,
        }
    }

    pub fn from_index(i: usize) -> Option<UnitKind> {
        ALL_KINDS.get(i).copied()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Unit {
    pub kind: UnitKind,
    pub hp: u32,
}

impl Unit {
    pub fn new(kind: UnitKind) -> Self {
        Unit {
            kind,
            hp: kind.hp(),
        }
    }

    /// Rough battle-value heuristic used by the AI and the UI.
    pub fn power(self) -> u32 {
        self.hp * self.kind.atk()
    }
}

pub const MAX_GARRISON: usize = 40;
