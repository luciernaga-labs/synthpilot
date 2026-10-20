use super::catalog::{Addressing, Block, Entry, Mapping};
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

pub(super) const HIGH: u8 = 0x31;
pub(super) const NAME_LENGTH: usize = 20;

const BLOCK: Block = Block::new(
    HIGH,
    Addressing::Part,
    "MIDI Parameter Change Table (Performance Part)",
);

const MIX: &str = "General";
const ENVELOPE: &str = "Part Envelope Offsets";
const FILTER: &str = "Part Filter Offsets";

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
];

pub(super) fn entries(engine: EngineId, drum: bool) -> Vec<Entry> {
    ROWS.iter()
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
        .collect()
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
        let ids: Vec<_> = entries(engine, true)
            .iter()
            .map(|entry| entry.definition.id)
            .collect();
        assert!(!ids.contains(&NOTE_SHIFT));
        assert!(!ids.contains(&AEG_RELEASE_TIME));
        assert_eq!(ids.len(), ROWS.len() - 2);
        assert_eq!(entries(engine, false).len(), ROWS.len());
    }

    #[test]
    fn entries_belong_to_the_requested_engine() {
        let engine = EngineId::new("awm2");
        assert!(
            entries(engine, false)
                .iter()
                .all(|entry| entry.definition.engine == engine && entry.mapping.is_some())
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
}
