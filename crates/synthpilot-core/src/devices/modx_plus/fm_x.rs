use super::PART_COUNT;
use super::catalog::{self, Addressing, Block, Entry, Mapping};
use super::part;
use crate::domain::{
    EncodeError, EngineId, EngineProfile, Layout, Location, ParameterChange, ParameterDefinition,
    ParameterId, Scope, ValueFormat, ValueRange,
};
use crate::protocol::SysExMessage;
use crate::protocol::yamaha::DeviceNumber;

pub const ENGINE_ID: EngineId = EngineId::new("fm-x");
pub const OPERATOR_COUNT: u8 = 8;

pub const ALGORITHM: ParameterId = ParameterId::new("fm-x.algorithm");
pub const FEEDBACK_LEVEL: ParameterId = ParameterId::new("fm-x.feedback-level");
pub const FILTER_TYPE: ParameterId = ParameterId::new("fm-x.filter-type");
pub const FILTER_CUTOFF: ParameterId = ParameterId::new("fm-x.filter-cutoff");
pub const FILTER_RESONANCE: ParameterId = ParameterId::new("fm-x.filter-resonance");
pub const FREQUENCY_MODE: ParameterId = ParameterId::new("fm-x.operator.frequency-mode");
pub const TUNE_COARSE: ParameterId = ParameterId::new("fm-x.operator.tune-coarse");
pub const TUNE_FINE: ParameterId = ParameterId::new("fm-x.operator.tune-fine");
pub const DETUNE: ParameterId = ParameterId::new("fm-x.operator.detune");
pub const OPERATOR_LEVEL: ParameterId = ParameterId::new("fm-x.operator.level");
pub const LEVEL_VELOCITY_SENSITIVITY: ParameterId =
    ParameterId::new("fm-x.operator.level-velocity-sensitivity");
pub const AEG_ATTACK_TIME: ParameterId = ParameterId::new("fm-x.operator.aeg-attack-time");
pub const AEG_DECAY_1_TIME: ParameterId = ParameterId::new("fm-x.operator.aeg-decay-1-time");
pub const AEG_RELEASE_TIME: ParameterId = ParameterId::new("fm-x.operator.aeg-release-time");
pub const LEVEL_SCALING_BREAK_POINT: ParameterId =
    ParameterId::new("fm-x.operator.level-scaling-break-point");

const COMMON: Block = Block::new(
    0x48,
    Addressing::Part,
    "MIDI Parameter Change Table (FM Part Common)",
);
const OPERATOR: Block = Block::new(
    0x49,
    Addressing::Element,
    "MIDI Parameter Change Table (FM Part Operator)",
);
const OPERATOR_PAGE: u16 = 220;
const FIRST_OPERATOR_LEVEL: i32 = 99;

const FREQUENCY_MODES: &[&str] = &["Ratio", "Fixed"];

const fn common(
    id: ParameterId,
    name: &'static str,
    category: &'static str,
    range: ValueRange,
) -> ParameterDefinition {
    ParameterDefinition::new(id, name, ENGINE_ID, category, Scope::Part, range)
}

const fn operator(
    id: ParameterId,
    name: &'static str,
    category: &'static str,
    range: ValueRange,
) -> ParameterDefinition {
    ParameterDefinition::new(id, name, ENGINE_ID, category, Scope::Element, range)
}

const fn operator_byte(low: u8, raw: (u32, u32)) -> Mapping {
    Mapping::new(OPERATOR, low, 1, raw, OPERATOR_PAGE)
}

const ENTRIES: &[Entry] = &[
    Entry::verified(
        common(ALGORITHM, "Algorithm", "Algorithm", ValueRange::new(1, 88)).with_default(1)
        .with_help("How the 8 Operators are connected. Operators at the bottom of the diagram are heard (carriers); the ones above shape their tone (modulators)."),
        Mapping::new(COMMON, 0x4F, 1, (0x00, 0x57), 220),
    ),
    Entry::verified(
        common(
            FEEDBACK_LEVEL,
            "Feedback Level",
            "Algorithm",
            ValueRange::new(0, 7),
        )
        .with_default(0)
        .with_help("Feeds an Operator back into itself. Higher values add brightness and grit, up to a noisy buzz."),
        Mapping::new(COMMON, 0x50, 1, (0x00, 0x07), 220),
    ),
    Entry::unverified(
        common(FILTER_TYPE, "Filter Type", "Filter", ValueRange::new(0, 21))
        .with_help("The kind of filter that shapes the tone of this Part."),
        "Yamaha's documentation doesn't make clear which value selects each filter type, so SynthPilot can't set it safely yet. Change it on the keyboard for now.",
    ),
    Entry::verified(
        common(
            FILTER_CUTOFF,
            "Filter Cutoff Frequency",
            "Filter",
            ValueRange::new(0, 255),
        )
        .with_default(255)
        .with_help("Sets the tone. With a low-pass filter, lower values sound darker and higher values brighter."),
        Mapping::new(COMMON, 0x0C, 2, (0x00, 0xFF), 219),
    ),
    Entry::verified(
        common(
            FILTER_RESONANCE,
            "Filter Resonance/Width",
            "Filter",
            ValueRange::new(0, 127),
        )
        .with_default(10)
        .with_help("Resonance emphasizes the frequencies around the cutoff for a sharper, more nasal tone. With band filters it sets the width of the band instead."),
        Mapping::new(COMMON, 0x0F, 1, (0x00, 0x7F), 219),
    ),
    Entry::verified(
        operator(
            FREQUENCY_MODE,
            "Oscillator Frequency Mode",
            "Oscillator",
            ValueRange::new(0, 1),
        )
        .with_format(ValueFormat::Choice(FREQUENCY_MODES))
        .with_default(0)
        .with_help("Ratio: the pitch follows the keys you play. Fixed: the same frequency on every key, useful for noises and percussive attacks."),
        operator_byte(0x03, (0x00, 0x01)),
    ),
    Entry::verified(
        operator(
            TUNE_COARSE,
            "Tune Coarse",
            "Oscillator",
            ValueRange::new(0, 31),
        )
        .with_default(1)
        .with_help("Sets this Operator's pitch relative to the note you play (in Ratio mode). On a modulator, it changes the character of the tone."),
        operator_byte(0x04, (0x00, 0x1F)),
    ),
    Entry::verified(
        operator(
            TUNE_FINE,
            "Tune Fine",
            "Oscillator",
            ValueRange::new(0, 127),
        )
        .with_default(0)
        .with_help("Fine adjustment of this Operator's pitch relative to the note you play (in Ratio mode)."),
        operator_byte(0x05, (0x00, 0x7F)),
    ),
    Entry::verified(
        operator(DETUNE, "Detune", "Oscillator", ValueRange::new(-15, 15))
            .with_format(ValueFormat::Signed)
            .with_default(0)
        .with_help("Slightly detunes this Operator for a gentle chorus or beating effect."),
        operator_byte(0x06, (0x00, 0x1E)),
    ),
    Entry::verified(
        operator(
            OPERATOR_LEVEL,
            "Operator Level",
            "Level",
            ValueRange::new(0, 99),
        )
        .with_default(0)
        .with_help("For a carrier: its volume. For a modulator: how much it changes the tone; higher is brighter."),
        operator_byte(0x1A, (0x00, 0x63)),
    ),
    Entry::verified(
        operator(
            LEVEL_VELOCITY_SENSITIVITY,
            "Level Velocity Sensitivity",
            "Level",
            ValueRange::new(-7, 7),
        )
        .with_format(ValueFormat::Signed)
        .with_default(0)
        .with_help("How much playing harder raises this Operator's level. On a modulator, harder playing sounds brighter."),
        operator_byte(0x20, (0x00, 0x0E)),
    ),
    Entry::verified(
        operator(
            AEG_ATTACK_TIME,
            "AEG Attack Time",
            "Level",
            ValueRange::new(0, 99),
        )
        .with_default(0)
        .with_help("How long this Operator takes to reach full level after you press a key. On a modulator, it shapes how the tone changes at the start of the note."),
        operator_byte(0x14, (0x00, 0x63)),
    ),
    Entry::verified(
        operator(
            AEG_DECAY_1_TIME,
            "AEG Decay 1 Time",
            "Level",
            ValueRange::new(0, 99),
        )
        .with_default(0)
        .with_help("How quickly this Operator's level falls after the attack. On a modulator, the tone changes instead of the volume."),
        operator_byte(0x15, (0x00, 0x63)),
    ),
    Entry::verified(
        operator(
            AEG_RELEASE_TIME,
            "AEG Release Time",
            "Level",
            ValueRange::new(0, 99),
        )
        .with_default(40)
        .with_help("How long this Operator keeps sounding after you release the key."),
        operator_byte(0x17, (0x00, 0x63)),
    ),
    Entry::unverified(
        operator(
            LEVEL_SCALING_BREAK_POINT,
            "Level Scaling Break Point",
            "Level",
            ValueRange::new(0, 99),
        )
        .with_help("The note where this Operator's level starts to change across the keyboard."),
        "Yamaha's documentation doesn't say which value matches each note (A-1 to C8), so SynthPilot can't set it yet. Change it on the keyboard for now.",
    ),
];

pub(super) fn entries() -> Vec<Entry> {
    let mut entries = part::entries(ENGINE_ID, false);
    entries.extend_from_slice(ENTRIES);
    entries
}

pub struct FmX {
    device_number: DeviceNumber,
    entries: Vec<Entry>,
    parameters: Vec<ParameterDefinition>,
}

impl FmX {
    pub fn new(device_number: DeviceNumber) -> Self {
        let entries = entries();
        Self {
            device_number,
            parameters: catalog::definitions(&entries),
            entries,
        }
    }
}

impl EngineProfile for FmX {
    fn id(&self) -> EngineId {
        ENGINE_ID
    }

    fn name(&self) -> &'static str {
        "FM-X"
    }

    fn description(&self) -> &'static str {
        "FM Part with 8 Operators combined by one of 88 Algorithms."
    }

    fn element_label(&self) -> &'static str {
        "Operator"
    }

    fn layout(&self) -> Layout {
        Layout {
            parts: PART_COUNT,
            elements: OPERATOR_COUNT,
        }
    }

    fn parameters(&self) -> &[ParameterDefinition] {
        &self.parameters
    }

    fn default_value(&self, parameter: &ParameterDefinition, location: Location) -> Option<i32> {
        if parameter.id == OPERATOR_LEVEL && location.element_number() == Some(1) {
            return Some(FIRST_OPERATOR_LEVEL);
        }
        parameter.default
    }

    fn encode(&self, change: &ParameterChange) -> Result<SysExMessage, EncodeError> {
        catalog::encode(&self.entries, self.layout(), self.device_number, change)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: ParameterId) -> Entry {
        let mut entry = *ENTRIES
            .iter()
            .find(|entry| entry.definition.id == id)
            .unwrap();
        assert!(entry.definition.help.is_some());
        entry.definition.help = None;
        entry
    }

    #[test]
    fn catalog_entries_match_their_data_list_rows() {
        let algorithm = Entry::verified(
            common(ALGORITHM, "Algorithm", "Algorithm", ValueRange::new(1, 88)).with_default(1),
            Mapping::new(COMMON, 0x4F, 1, (0x00, 0x57), 220),
        );
        assert_eq!(entry(ALGORITHM), algorithm);

        let detune = Entry::verified(
            operator(DETUNE, "Detune", "Oscillator", ValueRange::new(-15, 15))
                .with_format(ValueFormat::Signed)
                .with_default(0),
            operator_byte(0x06, (0x00, 0x1E)),
        );
        assert_eq!(entry(DETUNE), detune);
    }

    #[test]
    fn describes_itself() {
        let engine = FmX::new(DeviceNumber::default());
        assert_eq!(engine.name(), "FM-X");
        assert_ne!(engine.description(), "");
    }
}
