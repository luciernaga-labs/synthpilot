use super::catalog::{Addressing, Block, Entry, Mapping};
use super::effect_parameters::{
    self, Group, INSERTION_A_IDS, INSERTION_A_SLOT_NAMES, INSERTION_B_IDS, INSERTION_B_SLOT_NAMES,
    List, SLOTS,
};
use super::effects::{
    self, EFFECT_GROUPS, EFFECT_TABLE, INSERTION_B_GROUPS, INSERTION_B_LABELS, INSERTION_B_TABLE,
    INSERTION_LABELS,
};
use crate::domain::{
    ChoiceGroup, EngineId, ParameterDefinition, ParameterId, Part, Scope, Unit, ValueFormat,
    ValueRange,
};

pub const VOLUME: ParameterId = ParameterId::new("part.volume");
pub const PAN: ParameterId = ParameterId::new("part.pan");
pub const NOTE_SHIFT: ParameterId = ParameterId::new("part.note-shift");
pub const AEG_ATTACK_TIME: ParameterId = ParameterId::new("part.aeg-attack-time");
pub const AEG_DECAY_TIME: ParameterId = ParameterId::new("part.aeg-decay-time");
pub const AEG_RELEASE_TIME: ParameterId = ParameterId::new("part.aeg-release-time");
pub const FILTER_CUTOFF: ParameterId = ParameterId::new("part.filter-cutoff");
pub const FILTER_RESONANCE: ParameterId = ParameterId::new("part.filter-resonance");
pub const INSERTION_SWITCH: ParameterId = ParameterId::new("part.insertion-switch");
pub const INSERTION_CONNECTION: ParameterId = ParameterId::new("part.insertion-connection");
pub const INSERTION_A_TYPE: ParameterId = ParameterId::new("part.insertion-a-type");
pub const INSERTION_B_TYPE: ParameterId = ParameterId::new("part.insertion-b-type");
pub const VELOCITY_DEPTH: ParameterId = ParameterId::new("part.velocity-depth");
pub const VELOCITY_OFFSET: ParameterId = ParameterId::new("part.velocity-offset");
pub const REVERB_SEND: ParameterId = ParameterId::new("part.reverb-send");
pub const VARIATION_SEND: ParameterId = ParameterId::new("part.variation-send");
pub const DRY_LEVEL: ParameterId = ParameterId::new("part.dry-level");
pub const EQ_LOW_GAIN: ParameterId = ParameterId::new("part.eq-low-gain");
pub const EQ_MID_GAIN: ParameterId = ParameterId::new("part.eq-mid-gain");
pub const EQ_HIGH_GAIN: ParameterId = ParameterId::new("part.eq-high-gain");
pub const EQ_FREQUENCIES: ParameterId = ParameterId::new("part.eq-frequencies");
pub const MAIN_CATEGORY: ParameterId = ParameterId::new("part.main-category");
pub const SUB_CATEGORY: ParameterId = ParameterId::new("part.sub-category");
pub const VELOCITY_LIMIT_LOW: ParameterId = ParameterId::new("part.velocity-limit-low");
pub const VELOCITY_LIMIT_HIGH: ParameterId = ParameterId::new("part.velocity-limit-high");
pub const NOTE_LIMIT_LOW: ParameterId = ParameterId::new("part.note-limit-low");
pub const NOTE_LIMIT_HIGH: ParameterId = ParameterId::new("part.note-limit-high");
pub const PITCH_BEND_UPPER: ParameterId = ParameterId::new("part.pitch-bend-upper");
pub const PITCH_BEND_LOWER: ParameterId = ParameterId::new("part.pitch-bend-lower");
pub const PORTAMENTO_SWITCH: ParameterId = ParameterId::new("part.portamento-switch");
pub const PORTAMENTO_TIME: ParameterId = ParameterId::new("part.portamento-time");
pub const PORTAMENTO_MODE: ParameterId = ParameterId::new("part.portamento-mode");
pub const KEY_ASSIGN: ParameterId = ParameterId::new("part.key-assign");
pub const MONO_POLY: ParameterId = ParameterId::new("part.mono-poly");
pub const PART_OUTPUT: ParameterId = ParameterId::new("part.output");
pub const ELEMENT_PAN_SWITCH: ParameterId = ParameterId::new("part.element-pan-switch");
pub const ARP_PLAY_ONLY: ParameterId = ParameterId::new("part.arp-play-only");
pub const PART_SWITCH: ParameterId = ParameterId::new("part.switch");
pub const KEYBOARD_CONTROL: ParameterId = ParameterId::new("part.keyboard-control");
pub const MUTE: ParameterId = ParameterId::new("part.mute");

pub(super) const HIGH: u8 = 0x31;
pub(super) const NAME_LENGTH: usize = 20;

const TABLE: &str = "MIDI Parameter Change Table (Performance Part)";
const BLOCK: Block = Block::new(HIGH, Addressing::PART, TABLE);
const INSERTION_A: Block = Block::new(HIGH, Addressing::Part { base: 0x20 }, TABLE);
const INSERTION_B: Block = Block::new(HIGH, Addressing::Part { base: 0x30 }, TABLE);
const ARPEGGIO: Block = Block::new(HIGH, Addressing::Part { base: 0x60 }, TABLE);

const INSERTION_A_SLOTS: Group = Group {
    list: List::Variation,
    control: INSERTION_A_TYPE,
    kinds: &EFFECT_TABLE,
    ids: &INSERTION_A_IDS,
    names: &INSERTION_A_SLOT_NAMES,
    block: INSERTION_A,
    category: "Insertion A",
    scope: Scope::Part,
    pages: (209, 209),
};

const INSERTION_B_SLOTS: Group = Group {
    list: List::Variation,
    control: INSERTION_B_TYPE,
    kinds: &INSERTION_B_TABLE,
    ids: &INSERTION_B_IDS,
    names: &INSERTION_B_SLOT_NAMES,
    block: INSERTION_B,
    category: "Insertion B",
    scope: Scope::Part,
    pages: (209, 209),
};

const GROUPS: [&Group; 2] = [&INSERTION_A_SLOTS, &INSERTION_B_SLOTS];

pub const INSERTION_A_PARAMETERS: [ParameterId; SLOTS] = INSERTION_A_IDS;
pub const INSERTION_B_PARAMETERS: [ParameterId; SLOTS] = INSERTION_B_IDS;

pub(super) fn refine(parameter: &ParameterDefinition, part: &Part) -> Option<ParameterDefinition> {
    effect_parameters::refine(&GROUPS, parameter, part)
}

pub(super) fn dependents(
    parameters: &[ParameterDefinition],
    control: &ParameterDefinition,
    value: i32,
) -> Vec<ParameterDefinition> {
    effect_parameters::dependents(&GROUPS, parameters, control, value)
}

pub(super) fn reshapes(parameter: ParameterId) -> bool {
    GROUPS.iter().any(|group| group.control == parameter)
}

const MIX: &str = "General";
const ENVELOPE: &str = "Part Envelope Offsets";
const FILTER: &str = "Part Filter Offsets";
const EFFECTS: &str = "Effects";
const EQ: &str = "Part EQ";
const CATEGORY: &str = "Category";
const PLAY: &str = "Play Mode";
const RANGE: &str = "Key and Velocity Range";
const PITCH: &str = "Pitch";

const OFF_ON: &[&str] = &["Off", "On"];
const MONO_POLY_MODES: &[&str] = &["Mono", "Poly"];
const KEY_ASSIGN_MODES: &[&str] = &["Single", "Multi"];
const PORTAMENTO_MODES: &[&str] = &["Fingered", "Full-time"];

/// Performance Category List, page 18, in the order of the Part Main Category
/// data range (`00`–`10`, default `10` = No Assign), page 207.
pub const MAIN_CATEGORIES: &[&str] = &[
    "Piano",
    "Keyboard",
    "Organ",
    "Guitar",
    "Bass",
    "Strings",
    "Brass",
    "Woodwind",
    "Syn Lead",
    "Pad/Choir",
    "Syn Comp",
    "Chromatic Perc",
    "Drum / Perc",
    "Sound FX",
    "Musical FX",
    "Ethnic",
    "No Assign",
];

/// Part Output Select, page 207: 0 MainL&R, 9–12 USB pairs, 66–73 single USB
/// outputs, 125 Off and, for Drum Parts only, 127 Drum.
const OUTPUT_LABELS: [&str; 15] = [
    "MainL&R", "USB1&2", "USB3&4", "USB5&6", "USB7&8", "USB1", "USB2", "USB3", "USB4", "USB5",
    "USB6", "USB7", "USB8", "Off", "Drum",
];
const OUTPUT_VALUES: [u32; 15] = [0, 9, 10, 11, 12, 66, 67, 68, 69, 70, 71, 72, 73, 125, 127];

const INSERTION_SWITCHES: &[&str] = &["Both off", "A on", "B on", "A and B on"];
const CONNECTIONS: &[&str] = &["Parallel", "Ins A->B", "Ins B->A"];
const EQ_GAIN: ValueFormat = ValueFormat::Scaled {
    thousandths: 375,
    decimals: 2,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Normal,
    Fm,
    Drum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Row {
    id: ParameterId,
    name: &'static str,
    category: &'static str,
    range: ValueRange,
    default: i32,
    format: ValueFormat,
    unit: Option<Unit>,
    low: u8,
    raw: (u32, u32),
    page: u16,
    drum: bool,
    help: &'static str,
}

const fn level(
    id: ParameterId,
    name: &'static str,
    low: u8,
    default: i32,
    help: &'static str,
) -> Row {
    Row {
        id,
        name,
        category: EFFECTS,
        range: ValueRange::new(0, 127),
        default,
        format: ValueFormat::Number,
        unit: None,
        low,
        raw: (0x00, 0x7F),
        page: 207,
        drum: true,
        help,
    }
}

const fn eq_gain(id: ParameterId, name: &'static str, low: u8, help: &'static str) -> Row {
    Row {
        id,
        name,
        category: EQ,
        range: ValueRange::new(-32, 32),
        default: 0,
        format: EQ_GAIN,
        unit: Some(Unit::Decibels),
        low,
        raw: (0x20, 0x60),
        page: 208,
        drum: true,
        help,
    }
}

const fn offset(
    id: ParameterId,
    name: &'static str,
    category: &'static str,
    low: u8,
    help: &'static str,
) -> Row {
    Row {
        id,
        name,
        category,
        range: ValueRange::new(-64, 63),
        default: 0,
        format: ValueFormat::Signed,
        unit: None,
        low,
        raw: (0x00, 0x7F),
        page: 208,
        drum: true,
        help,
    }
}

const ROWS: &[Row] = &[
    Row {
        id: VOLUME,
        name: "Volume",
        category: MIX,
        range: ValueRange::new(0, 127),
        default: 100,
        format: ValueFormat::Number,
        unit: None,
        low: 0x24,
        raw: (0x00, 0x7F),
        page: 207,
        drum: true,
        help: "Overall volume of this Part in the Performance. 0 is silent.",
    },
    Row {
        id: PAN,
        name: "Pan",
        category: MIX,
        range: ValueRange::new(-63, 63),
        default: 0,
        format: ValueFormat::Pan,
        unit: None,
        low: 0x25,
        raw: (0x01, 0x7F),
        page: 207,
        drum: true,
        help: "Where this Part sits in the stereo field, from L63 (left) through C (center) to R63 (right).",
    },
    Row {
        id: NOTE_SHIFT,
        name: "Note Shift",
        category: MIX,
        range: ValueRange::new(-24, 24),
        default: 0,
        format: ValueFormat::Signed,
        unit: Some(Unit::Semitones),
        low: 0x2D,
        raw: (0x28, 0x58),
        page: 207,
        drum: false,
        help: "Transposes this Part up or down in semitones, up to 2 octaves, without changing what you play.",
    },
    Row {
        id: VELOCITY_DEPTH,
        name: "Velocity Sensitivity Depth",
        category: MIX,
        range: ValueRange::new(0, 127),
        default: 64,
        format: ValueFormat::Number,
        unit: None,
        low: 0x22,
        raw: (0x00, 0x7F),
        page: 207,
        drum: true,
        help: "How much your playing strength changes the velocity this Part receives. Higher values give more contrast between soft and hard playing.",
    },
    Row {
        id: VELOCITY_OFFSET,
        name: "Velocity Sensitivity Offset",
        category: MIX,
        range: ValueRange::new(0, 127),
        default: 64,
        format: ValueFormat::Number,
        unit: None,
        low: 0x23,
        raw: (0x00, 0x7F),
        page: 207,
        drum: true,
        help: "Shifts every velocity up or down. Lower it to make the Part softer and less bright overall.",
    },
    Row {
        drum: true,
        ..switch(
            PART_SWITCH,
            "Part Switch",
            MIX,
            OFF_ON,
            0x16,
            1,
            "Turns this Part on or off. Off silences it completely.",
        )
    },
    Row {
        drum: true,
        ..switch(
            KEYBOARD_CONTROL,
            "Keyboard Control",
            MIX,
            OFF_ON,
            0x17,
            0,
            "On makes this Part play from the keys together with the other Parts that have it on. Parts 9 to 16 only play over MIDI: the keyboard keeps it Off for them.",
        )
    },
    Row {
        drum: true,
        ..switch(
            MUTE,
            "Mute",
            MIX,
            OFF_ON,
            0x18,
            0,
            "On mutes this Part in the mix without turning it off.",
        )
    },
    Row {
        id: MAIN_CATEGORY,
        name: "Part Category Main",
        category: CATEGORY,
        range: ValueRange::new(0, 16),
        default: 16,
        format: ValueFormat::Choice(MAIN_CATEGORIES),
        unit: None,
        low: 0x14,
        raw: (0x00, 0x10),
        page: 207,
        drum: true,
        help: "Category the keyboard files this Part under, such as Piano, Pad/Choir or Chromatic Perc (bells and mallets). It only labels the sound; it doesn't change how it plays.",
    },
    switch(
        MONO_POLY,
        "Mono/Poly",
        PLAY,
        MONO_POLY_MODES,
        0x36,
        1,
        "Poly plays chords. Mono plays one note at a time, as on a lead or bass synth.",
    ),
    switch(
        KEY_ASSIGN,
        "Key Assign",
        PLAY,
        KEY_ASSIGN_MODES,
        0x35,
        1,
        "Multi lets a repeated note ring over itself. Single cuts the previous instance of the same note first.",
    ),
    switch(
        PORTAMENTO_SWITCH,
        "Portamento Switch",
        PLAY,
        OFF_ON,
        0x31,
        0,
        "Glides the pitch from one note to the next.",
    ),
    Row {
        id: PORTAMENTO_TIME,
        name: "Portamento Time",
        category: PLAY,
        range: ValueRange::new(0, 127),
        default: 64,
        format: ValueFormat::Number,
        unit: None,
        low: 0x32,
        raw: (0x00, 0x7F),
        page: 207,
        drum: false,
        help: "How long the glide between notes takes when Portamento is on.",
    },
    switch(
        PORTAMENTO_MODE,
        "Portamento Mode",
        PLAY,
        PORTAMENTO_MODES,
        0x33,
        1,
        "Fingered glides only when you play legato. Full-time glides on every note.",
    ),
    Row {
        drum: true,
        ..switch(
            ELEMENT_PAN_SWITCH,
            "Element Pan",
            MIX,
            OFF_ON,
            0x45,
            1,
            "On uses each Element's own pan setting. Off places every Element at the Part's Pan.",
        )
    },
    range_limit(
        VELOCITY_LIMIT_LOW,
        "Velocity Limit Low",
        0x1C,
        1,
        ValueFormat::Number,
        "Softest playing that triggers this Part. Use it with another Part to switch sounds by how hard you play.",
    ),
    range_limit(
        VELOCITY_LIMIT_HIGH,
        "Velocity Limit High",
        0x1D,
        127,
        ValueFormat::Number,
        "Hardest playing that still triggers this Part.",
    ),
    range_limit(
        NOTE_LIMIT_LOW,
        "Note Limit Low",
        0x1E,
        0,
        ValueFormat::Note,
        "Lowest key this Part plays. Use it to split the keyboard between Parts.",
    ),
    range_limit(
        NOTE_LIMIT_HIGH,
        "Note Limit High",
        0x1F,
        127,
        ValueFormat::Note,
        "Highest key this Part plays.",
    ),
    pitch_bend(
        PITCH_BEND_UPPER,
        "Pitch Bend Range Upper",
        0x20,
        2,
        "How far the pitch bend wheel raises the pitch when pushed up, in semitones.",
    ),
    pitch_bend(
        PITCH_BEND_LOWER,
        "Pitch Bend Range Lower",
        0x21,
        -2,
        "How far the pitch bend wheel moves the pitch when pulled down, in semitones.",
    ),
    offset(
        AEG_ATTACK_TIME,
        "AEG Attack Time",
        ENVELOPE,
        0x46,
        "Makes the start of every note faster (-) or slower (+) for the whole Part. 0 keeps each Element's or Operator's own setting.",
    ),
    offset(
        AEG_DECAY_TIME,
        "AEG Decay Time",
        ENVELOPE,
        0x47,
        "Shortens (-) or lengthens (+) how long the sound takes to fall after the attack, for the whole Part. 0 keeps the original setting.",
    ),
    Row {
        drum: false,
        ..offset(
            AEG_RELEASE_TIME,
            "AEG Release Time",
            ENVELOPE,
            0x49,
            "Shortens (-) or lengthens (+) how long notes ring after you release the key. 0 keeps the original setting.",
        )
    },
    offset(
        FILTER_CUTOFF,
        "Filter Cutoff Frequency",
        FILTER,
        0x50,
        "Makes the whole Part darker (-) or brighter (+) without editing each Element. 0 keeps the original setting.",
    ),
    offset(
        FILTER_RESONANCE,
        "Filter Resonance/Width",
        FILTER,
        0x51,
        "Lowers (-) or raises (+) the filter's emphasis around the cutoff for the whole Part. 0 keeps the original setting.",
    ),
    Row {
        id: INSERTION_SWITCH,
        name: "Insertion FX Switch",
        category: EFFECTS,
        range: ValueRange::new(0, 3),
        default: 3,
        format: ValueFormat::Choice(INSERTION_SWITCHES),
        unit: None,
        low: 0x26,
        raw: (0x00, 0x03),
        page: 207,
        drum: true,
        help: "Turns this Part's Insertion Effects A and B on or off. When one is off, the sound passes through it unchanged.",
    },
    level(
        REVERB_SEND,
        "Reverb Send",
        0x29,
        12,
        "How much of this Part goes to the Performance's Reverb. 0 keeps the Part dry.",
    ),
    level(
        VARIATION_SEND,
        "Variation Send",
        0x2A,
        0,
        "How much of this Part goes to the Performance's Variation effect, such as a chorus or a delay.",
    ),
    level(
        DRY_LEVEL,
        "Dry Level",
        0x2C,
        127,
        "Level of this Part without Reverb and Variation. Lower it to hear more of the effects.",
    ),
    eq_gain(
        EQ_LOW_GAIN,
        "3-band EQ Low Gain",
        0x55,
        "Cuts (-) or boosts (+) the low frequencies of this Part. 0.00 dB leaves them unchanged.",
    ),
    eq_gain(
        EQ_MID_GAIN,
        "3-band EQ Mid Gain",
        0x59,
        "Cuts (-) or boosts (+) the middle frequencies of this Part. 0.00 dB leaves them unchanged.",
    ),
    eq_gain(
        EQ_HIGH_GAIN,
        "3-band EQ High Gain",
        0x5D,
        "Cuts (-) or boosts (+) the high frequencies of this Part. 0.00 dB leaves them unchanged.",
    ),
];

const fn switch(
    id: ParameterId,
    name: &'static str,
    category: &'static str,
    labels: &'static [&'static str],
    low: u8,
    default: i32,
    help: &'static str,
) -> Row {
    assert!(labels.len() == 2, "a switch has two settings");
    Row {
        id,
        name,
        category,
        range: ValueRange::new(0, 1),
        default,
        format: ValueFormat::Choice(labels),
        unit: None,
        low,
        raw: (0x00, 0x01),
        page: if low >= 0x44 { 208 } else { 207 },
        drum: false,
        help,
    }
}

const fn range_limit(
    id: ParameterId,
    name: &'static str,
    low: u8,
    default: i32,
    format: ValueFormat,
    help: &'static str,
) -> Row {
    let velocity = matches!(format, ValueFormat::Number);
    let (range, raw) = if velocity {
        (ValueRange::new(1, 127), (0x01, 0x7F))
    } else {
        (ValueRange::new(0, 127), (0x00, 0x7F))
    };
    Row {
        id,
        name,
        category: RANGE,
        range,
        default,
        format,
        unit: None,
        low,
        raw,
        page: 207,
        drum: true,
        help,
    }
}

const fn pitch_bend(
    id: ParameterId,
    name: &'static str,
    low: u8,
    default: i32,
    help: &'static str,
) -> Row {
    Row {
        id,
        name,
        category: PITCH,
        range: ValueRange::new(-48, 24),
        default,
        format: ValueFormat::Signed,
        unit: Some(Unit::Semitones),
        low,
        raw: (0x10, 0x58),
        page: 207,
        drum: true,
        help,
    }
}

const fn connection(kind: Kind) -> Row {
    let (range, raw) = match kind {
        Kind::Fm => (ValueRange::new(1, 2), (0x01, 0x02)),
        Kind::Normal | Kind::Drum => (ValueRange::new(0, 2), (0x00, 0x02)),
    };
    Row {
        id: INSERTION_CONNECTION,
        name: "Insertion Connection Type",
        category: EFFECTS,
        range,
        default: range.min(),
        format: ValueFormat::Choice(CONNECTIONS),
        unit: None,
        low: 0x3B,
        raw,
        page: 207,
        drum: true,
        help: match kind {
            Kind::Fm => {
                "Order of this Part's Insertion Effects: A then B, or B then A. FM-X Parts always chain them."
            }
            Kind::Normal | Kind::Drum => {
                "How this Part's Insertion Effects are connected: side by side (Parallel), A then B, or B then A."
            }
        },
    }
}

struct Insertion {
    id: ParameterId,
    name: &'static str,
    block: Block,
    labels: &'static [&'static str],
    table: &'static [u32],
    groups: &'static [ChoiceGroup],
    help: &'static str,
}

fn insertion_type(engine: EngineId, insertion: &Insertion) -> Entry {
    Entry::verified(
        effects::effect_type(
            insertion.id,
            insertion.name,
            engine,
            EFFECTS,
            Scope::Part,
            insertion.labels,
        )
        .with_groups(insertion.groups)
        .with_help(insertion.help),
        Mapping::listed(insertion.block, 0x00, 2, insertion.table, 209),
    )
}

fn output(engine: EngineId, kind: Kind) -> Entry {
    let (labels, table, help): (&'static [&'static str], &'static [u32], _) = match kind {
        Kind::Drum => (
            &OUTPUT_LABELS,
            &OUTPUT_VALUES,
            "Where this Part's sound goes: the main outputs, a USB audio channel, Off, or Drum to let each drum key choose its own output.",
        ),
        Kind::Normal | Kind::Fm => (
            &OUTPUT_LABELS[..14],
            &OUTPUT_VALUES[..14],
            "Where this Part's sound goes: the main outputs, a USB audio channel to record it separately on a computer, or Off.",
        ),
    };
    let last = i32::try_from(labels.len()).unwrap_or(1) - 1;
    Entry::verified(
        ParameterDefinition::new(
            PART_OUTPUT,
            "Part Output",
            engine,
            MIX,
            Scope::Part,
            ValueRange::new(0, last),
        )
        .with_format(ValueFormat::Choice(labels))
        .with_default(0)
        .with_help(help),
        Mapping::listed(BLOCK, 0x3F, 1, table, 207),
    )
}

pub(super) fn entries(engine: EngineId, kind: Kind) -> Vec<Entry> {
    let drum = kind == Kind::Drum;
    let mut entries: Vec<Entry> = ROWS
        .iter()
        .chain([connection(kind)].iter())
        .filter(|row| row.drum || !drum)
        .map(|row| {
            let mut definition = ParameterDefinition::new(
                row.id,
                row.name,
                engine,
                row.category,
                Scope::Part,
                row.range,
            )
            .with_default(row.default)
            .with_format(row.format)
            .with_help(row.help);
            if let Some(unit) = row.unit {
                definition = definition.with_unit(unit);
            }
            Entry::verified(
                definition,
                Mapping::new(BLOCK, row.low, 1, row.raw, row.page),
            )
        })
        .collect();
    entries.extend([
        insertion_type(
            engine,
            &Insertion {
                id: INSERTION_A_TYPE,
                name: "Insertion A Type",
                block: INSERTION_A,
                labels: &INSERTION_LABELS,
                table: &EFFECT_TABLE,
                groups: EFFECT_GROUPS,
                help: "Effect used as Insertion A on this Part only, grouped by category as on the keyboard. THRU passes the sound unchanged.",
            },
        ),
        insertion_type(
            engine,
            &Insertion {
                id: INSERTION_B_TYPE,
                name: "Insertion B Type",
                block: INSERTION_B,
                labels: &INSERTION_B_LABELS,
                table: &INSERTION_B_TABLE,
                groups: INSERTION_B_GROUPS,
                help: "Effect used as Insertion B on this Part only. THRU passes the sound unchanged. VINYL BREAK and the BEAT REPEAT types are only available on Insertion A.",
            },
        ),
        output(engine, kind),
        Entry::verified(
            ParameterDefinition::new(
                ARP_PLAY_ONLY,
                "Arp Play Only",
                engine,
                PLAY,
                Scope::Part,
                ValueRange::new(0, 1),
            )
            .with_format(ValueFormat::Choice(OFF_ON))
            .with_default(0)
            .with_help("On makes this Part sound only through the arpeggiator, not from the keys you play directly."),
            Mapping::new(ARPEGGIO, 0x03, 1, (0x00, 0x01), 210),
        ),
        Entry::unverified(
            ParameterDefinition::new(
                SUB_CATEGORY,
                "Part Category Sub",
                engine,
                CATEGORY,
                Scope::Part,
                ValueRange::new(0, 7),
            ),
            "Yamaha's Data List gives the Sub Category 8 values (00–07) but names up to 9 per Main Category, so SynthPilot can't tell which number is which yet. Change it on the keyboard for now.",
        ),
        Entry::unverified(
            ParameterDefinition::new(
                EQ_FREQUENCIES,
                "3-band EQ Frequencies and Mid Q",
                engine,
                EQ,
                Scope::Part,
                ValueRange::new(0, 0x7F),
            ),
            "Yamaha's documentation gives only the lowest and highest values, so SynthPilot can't set them safely yet. Change them on the keyboard for now.",
        ),
    ]);
    for group in GROUPS {
        entries.extend(group.entries(engine));
    }
    entries
}

pub(super) fn name(data: &[u8]) -> Option<String> {
    let bytes = data.get(..NAME_LENGTH)?;
    let name: String = bytes
        .iter()
        .take_while(|byte| **byte != 0)
        .filter(|byte| (0x20..=0x7E).contains(*byte))
        .map(|byte| char::from(*byte))
        .collect();
    let name = name.trim_end();
    (!name.is_empty()).then(|| name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drum_parts_skip_parameters_the_data_list_excludes() {
        let engine = EngineId::new("drum");
        let ids: Vec<_> = entries(engine, Kind::Drum)
            .iter()
            .map(|entry| entry.definition.id)
            .collect();
        assert!(!ids.contains(&NOTE_SHIFT));
        assert!(!ids.contains(&AEG_RELEASE_TIME));
        assert!(ids.contains(&INSERTION_CONNECTION));
        for id in [
            MONO_POLY,
            KEY_ASSIGN,
            PORTAMENTO_SWITCH,
            PORTAMENTO_TIME,
            PORTAMENTO_MODE,
        ] {
            assert!(!ids.contains(&id));
        }
        assert!(ids.contains(&ELEMENT_PAN_SWITCH));
        assert!(ids.contains(&NOTE_LIMIT_LOW));
        assert!(ids.contains(&PART_SWITCH));
        assert_eq!(ids.len(), entries(engine, Kind::Normal).len() - 7);
    }

    #[test]
    fn fm_parts_cannot_use_a_parallel_connection() {
        let range = |kind| {
            entries(EngineId::new("engine"), kind)
                .iter()
                .find(|entry| entry.definition.id == INSERTION_CONNECTION)
                .map(|entry| entry.definition.range)
        };
        assert_eq!(range(Kind::Fm), Some(ValueRange::new(1, 2)));
        assert_eq!(range(Kind::Normal), Some(ValueRange::new(0, 2)));
    }

    #[test]
    fn entries_belong_to_the_requested_engine() {
        let engine = EngineId::new("awm2");
        assert!(
            entries(engine, Kind::Normal)
                .iter()
                .all(|entry| entry.definition.engine == engine)
        );
    }

    #[test]
    fn names_are_read_from_ascii_bytes() {
        let mut data = b"CFX Concert".to_vec();
        data.resize(NAME_LENGTH, b' ');
        data.push(0x40);
        assert_eq!(name(&data).as_deref(), Some("CFX Concert"));

        let mut padded = b"Pad".to_vec();
        padded.resize(NAME_LENGTH, 0);
        assert_eq!(name(&padded).as_deref(), Some("Pad"));

        assert_eq!(name(&[0; NAME_LENGTH]), None);
        assert_eq!(name(b"short"), None);
    }

    #[test]
    fn row_builders_follow_the_data_list() {
        let send = level(REVERB_SEND, "Reverb Send", 0x29, 12, "Send");
        assert_eq!((send.low, send.raw, send.page), (0x29, (0x00, 0x7F), 207));
        let gain = eq_gain(EQ_LOW_GAIN, "Low", 0x55, "Gain");
        assert_eq!(
            (gain.raw, gain.range),
            ((0x20, 0x60), ValueRange::new(-32, 32))
        );
        assert_eq!(gain.unit, Some(Unit::Decibels));
        let drum = connection(Kind::Drum);
        assert_eq!((drum.range, drum.default), (ValueRange::new(0, 2), 0));
        let fm = connection(Kind::Fm);
        assert_eq!((fm.raw, fm.default), ((0x01, 0x02), 1));
        assert_eq!(offset(FILTER_CUTOFF, "Cutoff", FILTER, 0x50, "").page, 208);
        assert!(fm.help.contains("FM-X"));
    }
}
