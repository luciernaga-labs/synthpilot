use super::PART_COUNT;
use super::catalog::{self, Addressing, Block, Entry, Mapping};
use super::part;
use crate::domain::{
    EncodeError, EngineId, EngineProfile, Layout, Location, ParameterChange, ParameterDefinition,
    ParameterId, Scope, Unit, ValueFormat, ValueRange,
};
use crate::protocol::SysExMessage;
use crate::protocol::yamaha::DeviceNumber;

pub const ENGINE_ID: EngineId = EngineId::new("awm2");
pub const ELEMENT_COUNT: u8 = 8;

pub const ELEMENT_SWITCH: ParameterId = ParameterId::new("awm2.element-switch");
pub const XA_CONTROL: ParameterId = ParameterId::new("awm2.xa-control");
pub const WAVE_NUMBER: ParameterId = ParameterId::new("awm2.wave-number");
pub const ELEMENT_LEVEL: ParameterId = ParameterId::new("awm2.element-level");
pub const ELEMENT_PAN: ParameterId = ParameterId::new("awm2.element-pan");
pub const LEVEL_VELOCITY_SENSITIVITY: ParameterId =
    ParameterId::new("awm2.level-velocity-sensitivity");
pub const AEG_ATTACK_TIME: ParameterId = ParameterId::new("awm2.aeg-attack-time");
pub const AEG_DECAY_1_TIME: ParameterId = ParameterId::new("awm2.aeg-decay-1-time");
pub const AEG_DECAY_2_TIME: ParameterId = ParameterId::new("awm2.aeg-decay-2-time");
pub const AEG_RELEASE_TIME: ParameterId = ParameterId::new("awm2.aeg-release-time");
pub const COARSE_TUNE: ParameterId = ParameterId::new("awm2.coarse-tune");
pub const FINE_TUNE: ParameterId = ParameterId::new("awm2.fine-tune");
pub const FILTER_TYPE: ParameterId = ParameterId::new("awm2.filter-type");
pub const FILTER_CUTOFF: ParameterId = ParameterId::new("awm2.filter-cutoff");
pub const FILTER_RESONANCE: ParameterId = ParameterId::new("awm2.filter-resonance");

const TABLE: &str = "MIDI Parameter Change Table (Normal Part Element)";
const GROUP_1: Block = Block::new(0x41, Addressing::Element, TABLE);
const GROUP_2: Block = Block::new(0x42, Addressing::Element, TABLE);
const GROUP_1_PAGE: u16 = 217;
const GROUP_2_PAGE: u16 = 218;

const OFF_ON: &[&str] = &["Off", "On"];
const XA_MODES: &[&str] = &[
    "Normal", "Legato", "Key Off", "Cycle", "Random", "A.SW Off", "A.SW1 On", "A.SW2 On",
];

const fn element(
    id: ParameterId,
    name: &'static str,
    category: &'static str,
    range: ValueRange,
) -> ParameterDefinition {
    ParameterDefinition::new(id, name, ENGINE_ID, category, Scope::Element, range)
}

const fn group_1(low: u8, raw: (u32, u32)) -> Mapping {
    Mapping::new(GROUP_1, low, 1, raw, GROUP_1_PAGE)
}

const fn group_2(low: u8, size: usize, raw: (u32, u32)) -> Mapping {
    Mapping::new(GROUP_2, low, size, raw, GROUP_2_PAGE)
}

const ENTRIES: &[Entry] = &[
    Entry::verified(
        element(
            ELEMENT_SWITCH,
            "Element Switch",
            "Oscillator",
            ValueRange::new(0, 1),
        )
        .with_format(ValueFormat::Choice(OFF_ON))
        .with_help("Turns this Element on or off. An Element that is off makes no sound."),
        group_1(0x00, (0x00, 0x01)),
    ),
    Entry::verified(
        element(
            XA_CONTROL,
            "XA Control",
            "Oscillator",
            ValueRange::new(0, 7),
        )
        .with_format(ValueFormat::Choice(XA_MODES))
        .with_default(0)
        .with_help("Decides when this Element plays. Normal: always. Legato: only on notes played legato. Key Off: when you release the key. Cycle and Random: takes turns with other Elements. A.SW: depends on the Assignable Switches."),
        group_1(0x0C, (0x00, 0x07)),
    ),
    Entry::unverified(
        element(
            WAVE_NUMBER,
            "Wave Number",
            "Oscillator",
            ValueRange::new(1, 6347),
        )
        .with_help("The recorded sound (Wave) this Element plays."),
        "SynthPilot can't set the Wave yet because the number depends on the Wave bank. Change it on the keyboard for now.",
    ),
    Entry::verified(
        element(
            ELEMENT_LEVEL,
            "Element Level",
            "Amplitude",
            ValueRange::new(0, 127),
        )
        .with_default(127)
        .with_help("Volume of this Element within the Part."),
        group_1(0x28, (0x00, 0x7F)),
    ),
    Entry::verified(
        element(
            ELEMENT_PAN,
            "Element Pan",
            "Amplitude",
            ValueRange::new(-63, 63),
        )
        .with_format(ValueFormat::Pan)
        .with_default(0)
        .with_help("Stereo position of this Element, from L63 (left) through C (center) to R63 (right)."),
        group_1(0x08, (0x01, 0x7F)),
    ),
    Entry::verified(
        element(
            LEVEL_VELOCITY_SENSITIVITY,
            "Level Velocity Sensitivity",
            "Amplitude",
            ValueRange::new(-64, 63),
        )
        .with_format(ValueFormat::Signed)
        .with_default(0)
        .with_help("How much playing harder makes this Element louder. Positive: harder is louder. Negative: harder is softer. 0: same level however you play."),
        group_1(0x29, (0x00, 0x7F)),
    ),
    Entry::verified(
        element(
            AEG_ATTACK_TIME,
            "AEG Attack Time",
            "Amplitude",
            ValueRange::new(0, 127),
        )
        .with_default(0)
        .with_help("How long the sound takes to reach full volume after you press a key. Low: instant, like a piano. High: slow fade-in, like a pad."),
        group_1(0x2D, (0x00, 0x7F)),
    ),
    Entry::verified(
        element(
            AEG_DECAY_1_TIME,
            "AEG Decay 1 Time",
            "Amplitude",
            ValueRange::new(0, 127),
        )
        .with_default(64)
        .with_help("How quickly the volume falls right after the attack. First of two decay stages."),
        group_1(0x2E, (0x00, 0x7F)),
    ),
    Entry::verified(
        element(
            AEG_DECAY_2_TIME,
            "AEG Decay 2 Time",
            "Amplitude",
            ValueRange::new(0, 127),
        )
        .with_default(64)
        .with_help("How quickly the volume settles to the level it holds while you keep the key down. Second decay stage."),
        group_1(0x2F, (0x00, 0x7F)),
    ),
    Entry::verified(
        element(
            AEG_RELEASE_TIME,
            "AEG Release Time",
            "Amplitude",
            ValueRange::new(0, 127),
        )
        .with_default(50)
        .with_help("How long the sound keeps ringing after you release the key."),
        group_1(0x31, (0x00, 0x7F)),
    ),
    Entry::verified(
        element(
            COARSE_TUNE,
            "Coarse Tune",
            "Pitch",
            ValueRange::new(-48, 48),
        )
        .with_format(ValueFormat::Signed)
        .with_unit(Unit::Semitones)
        .with_default(0)
        .with_help("Changes this Element's pitch in semitones, up to 4 octaves up or down."),
        group_1(0x49, (0x10, 0x70)),
    ),
    Entry::verified(
        element(FINE_TUNE, "Fine Tune", "Pitch", ValueRange::new(-64, 63))
            .with_format(ValueFormat::Signed)
            .with_default(0)
        .with_help("Small pitch changes, for slight detuning between Elements."),
        group_1(0x4A, (0x00, 0x7F)),
    ),
    Entry::unverified(
        element(FILTER_TYPE, "Filter Type", "Filter", ValueRange::new(0, 21))
        .with_help("The kind of filter that shapes this Element's tone."),
        "Yamaha's documentation doesn't make clear which value selects each filter type, so SynthPilot can't set it safely yet. Change it on the keyboard for now.",
    ),
    Entry::verified(
        element(
            FILTER_CUTOFF,
            "Filter Cutoff Frequency",
            "Filter",
            ValueRange::new(0, 255),
        )
        .with_default(160)
        .with_help("Sets the tone. With a low-pass filter, lower values sound darker and higher values brighter."),
        group_2(0x01, 2, (0x00, 0xFF)),
    ),
    Entry::verified(
        element(
            FILTER_RESONANCE,
            "Filter Resonance/Width",
            "Filter",
            ValueRange::new(0, 127),
        )
        .with_default(0)
        .with_help("Resonance emphasizes the frequencies around the cutoff for a sharper, more nasal tone. With band filters it sets the width of the band instead."),
        group_2(0x05, 1, (0x00, 0x7F)),
    ),
];

pub(super) fn entries() -> Vec<Entry> {
    let mut entries = part::entries(ENGINE_ID, false);
    entries.extend_from_slice(ENTRIES);
    entries
}

pub struct Awm2 {
    device_number: DeviceNumber,
    entries: Vec<Entry>,
    parameters: Vec<ParameterDefinition>,
}

impl Awm2 {
    pub fn new(device_number: DeviceNumber) -> Self {
        let entries = entries();
        Self {
            device_number,
            parameters: catalog::definitions(&entries),
            entries,
        }
    }
}

impl EngineProfile for Awm2 {
    fn id(&self) -> EngineId {
        ENGINE_ID
    }

    fn name(&self) -> &'static str {
        "AWM2"
    }

    fn description(&self) -> &'static str {
        "Sample-based Part with up to 8 Elements, each playing its own Wave."
    }

    fn element_label(&self) -> &'static str {
        "Element"
    }

    fn layout(&self) -> Layout {
        Layout {
            parts: PART_COUNT,
            elements: ELEMENT_COUNT,
        }
    }

    fn parameters(&self) -> &[ParameterDefinition] {
        &self.parameters
    }

    fn default_value(&self, parameter: &ParameterDefinition, location: Location) -> Option<i32> {
        if parameter.id == ELEMENT_SWITCH {
            return Some(i32::from(location.element_number() == Some(1)));
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
        let level = Entry::verified(
            element(
                ELEMENT_LEVEL,
                "Element Level",
                "Amplitude",
                ValueRange::new(0, 127),
            )
            .with_default(127),
            group_1(0x28, (0x00, 0x7F)),
        );
        assert_eq!(entry(ELEMENT_LEVEL), level);

        let cutoff = Entry::verified(
            element(
                FILTER_CUTOFF,
                "Filter Cutoff Frequency",
                "Filter",
                ValueRange::new(0, 255),
            )
            .with_default(160),
            group_2(0x01, 2, (0x00, 0xFF)),
        );
        assert_eq!(entry(FILTER_CUTOFF), cutoff);
    }

    #[test]
    fn describes_itself() {
        let engine = Awm2::new(DeviceNumber::default());
        assert_eq!(engine.name(), "AWM2");
        assert_ne!(engine.description(), "");
    }
}
