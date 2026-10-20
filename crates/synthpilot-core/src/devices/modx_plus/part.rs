use super::catalog::{Addressing, Block, Entry, Mapping};
use super::effects::{self, EFFECT_TABLE, INSERTION_LABELS};
use crate::domain::{
    EngineId, ParameterDefinition, ParameterId, Scope, Unit, ValueFormat, ValueRange,
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
pub const INSERTION_PARAMETERS: ParameterId = ParameterId::new("part.insertion-parameters");
pub const REVERB_SEND: ParameterId = ParameterId::new("part.reverb-send");
pub const VARIATION_SEND: ParameterId = ParameterId::new("part.variation-send");
pub const DRY_LEVEL: ParameterId = ParameterId::new("part.dry-level");
pub const EQ_LOW_GAIN: ParameterId = ParameterId::new("part.eq-low-gain");
pub const EQ_MID_GAIN: ParameterId = ParameterId::new("part.eq-mid-gain");
pub const EQ_HIGH_GAIN: ParameterId = ParameterId::new("part.eq-high-gain");
pub const EQ_FREQUENCIES: ParameterId = ParameterId::new("part.eq-frequencies");

pub(super) const HIGH: u8 = 0x31;
pub(super) const NAME_LENGTH: usize = 20;

const TABLE: &str = "MIDI Parameter Change Table (Performance Part)";
const BLOCK: Block = Block::new(HIGH, Addressing::PART, TABLE);
const INSERTION_A: Block = Block::new(HIGH, Addressing::Part { base: 0x20 }, TABLE);
const INSERTION_B: Block = Block::new(HIGH, Addressing::Part { base: 0x30 }, TABLE);

const MIX: &str = "General";
const ENVELOPE: &str = "Part Envelope Offsets";
const FILTER: &str = "Part Filter Offsets";
const EFFECTS: &str = "Effects";
const EQ: &str = "Part EQ";

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

fn insertion_type(
    engine: EngineId,
    id: ParameterId,
    name: &'static str,
    block: Block,
    help: &'static str,
) -> Entry {
    Entry::verified(
        effects::effect_type(id, name, engine, EFFECTS, Scope::Part, &INSERTION_LABELS)
            .with_help(help),
        Mapping::listed(block, 0x00, 2, &EFFECT_TABLE, 209),
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
            INSERTION_A_TYPE,
            "Insertion A Type",
            INSERTION_A,
            "Effect used as Insertion A on this Part only. THRU passes the sound unchanged.",
        ),
        insertion_type(
            engine,
            INSERTION_B_TYPE,
            "Insertion B Type",
            INSERTION_B,
            "Effect used as Insertion B on this Part only. THRU passes the sound unchanged.",
        ),
        Entry::unverified(
            ParameterDefinition::new(
                INSERTION_PARAMETERS,
                "Insertion A/B Parameters 1–24",
                engine,
                EFFECTS,
                Scope::Part,
                ValueRange::new(0, 0x3FFF),
            ),
            "Each effect type has its own parameters, and SynthPilot doesn't know them yet. Change them on the keyboard for now.",
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
        assert_eq!(ids.len(), entries(engine, Kind::Normal).len() - 2);
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
