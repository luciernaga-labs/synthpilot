use super::PART_COUNT;
use super::catalog::{self, Addressing, Block, Entry, Mapping};
use super::part;
use crate::domain::{
    EncodeError, EngineId, EngineProfile, Layout, Location, ParameterChange, ParameterDefinition,
    ParameterId, Part, Scope, ValueFormat, ValueRange,
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

pub const PITCH_VELOCITY_SENSITIVITY: ParameterId =
    ParameterId::new("fm-x.pitch-velocity-sensitivity");
pub const FILTER_CUTOFF_VELOCITY: ParameterId = ParameterId::new("fm-x.filter-cutoff-velocity");
pub const FEG_HOLD_TIME: ParameterId = ParameterId::new("fm-x.feg-hold-time");
pub const FEG_ATTACK_TIME: ParameterId = ParameterId::new("fm-x.feg-attack-time");
pub const FEG_DECAY_1_TIME: ParameterId = ParameterId::new("fm-x.feg-decay-1-time");
pub const FEG_DECAY_2_TIME: ParameterId = ParameterId::new("fm-x.feg-decay-2-time");
pub const FEG_RELEASE_TIME: ParameterId = ParameterId::new("fm-x.feg-release-time");
pub const FEG_HOLD_LEVEL: ParameterId = ParameterId::new("fm-x.feg-hold-level");
pub const FEG_ATTACK_LEVEL: ParameterId = ParameterId::new("fm-x.feg-attack-level");
pub const FEG_DECAY_1_LEVEL: ParameterId = ParameterId::new("fm-x.feg-decay-1-level");
pub const FEG_DECAY_2_LEVEL: ParameterId = ParameterId::new("fm-x.feg-decay-2-level");
pub const FEG_RELEASE_LEVEL: ParameterId = ParameterId::new("fm-x.feg-release-level");
pub const FEG_DEPTH: ParameterId = ParameterId::new("fm-x.feg-depth");
pub const FEG_DEPTH_VELOCITY: ParameterId = ParameterId::new("fm-x.feg-depth-velocity");
pub const LFO_WAVE: ParameterId = ParameterId::new("fm-x.lfo-wave");
pub const LFO_SPEED: ParameterId = ParameterId::new("fm-x.lfo-speed");
pub const LFO_DELAY_TIME: ParameterId = ParameterId::new("fm-x.lfo-delay-time");
pub const LFO_PITCH: ParameterId = ParameterId::new("fm-x.lfo-pitch");
pub const LFO_AMPLITUDE: ParameterId = ParameterId::new("fm-x.lfo-amplitude");
pub const LFO_FILTER: ParameterId = ParameterId::new("fm-x.lfo-filter");
pub const SPECTRAL_FORM: ParameterId = ParameterId::new("fm-x.operator.spectral-form");
pub const SPECTRAL_SKIRT: ParameterId = ParameterId::new("fm-x.operator.spectral-skirt");
pub const SPECTRAL_RESONANCE: ParameterId = ParameterId::new("fm-x.operator.spectral-resonance");
pub const AEG_ATTACK_LEVEL: ParameterId = ParameterId::new("fm-x.operator.aeg-attack-level");
pub const AEG_DECAY_1_LEVEL: ParameterId = ParameterId::new("fm-x.operator.aeg-decay-1-level");
pub const AEG_DECAY_2_LEVEL: ParameterId = ParameterId::new("fm-x.operator.aeg-decay-2-level");
pub const AEG_RELEASE_LEVEL: ParameterId = ParameterId::new("fm-x.operator.aeg-release-level");
pub const AEG_DECAY_2_TIME: ParameterId = ParameterId::new("fm-x.operator.aeg-decay-2-time");
pub const AEG_HOLD_TIME: ParameterId = ParameterId::new("fm-x.operator.aeg-hold-time");
pub const AEG_TIME_KEY_FOLLOW: ParameterId = ParameterId::new("fm-x.operator.aeg-time-key-follow");
pub const LEVEL_SCALING_LOW_DEPTH: ParameterId =
    ParameterId::new("fm-x.operator.level-scaling-low-depth");
pub const LEVEL_SCALING_HIGH_DEPTH: ParameterId =
    ParameterId::new("fm-x.operator.level-scaling-high-depth");
pub const OPERATOR_LFO_PITCH: ParameterId = ParameterId::new("fm-x.operator.lfo-pitch");
pub const OPERATOR_LFO_AMPLITUDE: ParameterId = ParameterId::new("fm-x.operator.lfo-amplitude");

const COMMON: Block = Block::new(
    0x48,
    Addressing::PART,
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
const SPECTRAL_FORMS: &[&str] = &["Sine", "All 1", "All 2", "Odd 1", "Odd 2", "Res 1", "Res 2"];
const LFO_WAVES: &[&str] = &["Triangle", "Saw Down", "Saw Up", "Square", "Sine", "S/H"];
const COMMON_PAGE: u16 = 219;
const LFO_PAGE: u16 = 220;
const TIME: ValueRange = ValueRange::new(0, 127);
const SIGNED: ValueRange = ValueRange::new(-64, 63);
const EG_LEVEL: ValueRange = ValueRange::new(-128, 127);
const NINETY_NINE: ValueRange = ValueRange::new(0, 99);
const EIGHT: ValueRange = ValueRange::new(0, 7);

const fn row(
    definition: ParameterDefinition,
    default: i32,
    help: &'static str,
    mapping: Mapping,
) -> Entry {
    Entry::verified(definition.with_default(default).with_help(help), mapping)
}

const fn common_byte(low: u8, raw: (u32, u32), page: u16) -> Mapping {
    Mapping::new(COMMON, low, 1, raw, page)
}

const fn eg_level(id: ParameterId, name: &'static str) -> ParameterDefinition {
    common(id, name, "Filter EG", EG_LEVEL).with_format(ValueFormat::Signed)
}

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
    row(
        common(PITCH_VELOCITY_SENSITIVITY, "Pitch Velocity Sensitivity", "Pitch", SIGNED)
            .with_format(ValueFormat::Signed),
        0,
        "How much playing harder raises (+) or lowers (-) the pitch of the whole Part.",
        common_byte(0x06, (0x00, 0x7F), COMMON_PAGE),
    ),
    row(
        common(FILTER_CUTOFF_VELOCITY, "Filter Cutoff Velocity Sensitivity", "Filter", SIGNED)
            .with_format(ValueFormat::Signed),
        0,
        "How much playing harder makes the Part brighter (+) or darker (-).",
        common_byte(0x0E, (0x00, 0x7F), COMMON_PAGE),
    ),
    row(
        common(FEG_HOLD_TIME, "FEG Hold Time", "Filter EG", TIME),
        0,
        "How long the filter stays at its starting point before the attack.",
        common_byte(0x17, (0x00, 0x7F), COMMON_PAGE),
    ),
    row(
        common(FEG_ATTACK_TIME, "FEG Attack Time", "Filter EG", TIME),
        0,
        "How fast the filter moves to the Attack Level after you press a key.",
        common_byte(0x18, (0x00, 0x7F), COMMON_PAGE),
    ),
    row(
        common(FEG_DECAY_1_TIME, "FEG Decay 1 Time", "Filter EG", TIME),
        0,
        "How fast the filter moves from the Attack Level to the Decay 1 Level.",
        common_byte(0x19, (0x00, 0x7F), COMMON_PAGE),
    ),
    row(
        common(FEG_DECAY_2_TIME, "FEG Decay 2 Time", "Filter EG", TIME),
        0,
        "How fast the filter moves to the Decay 2 Level, where it stays while you hold the key.",
        common_byte(0x1A, (0x00, 0x7F), COMMON_PAGE),
    ),
    row(
        common(FEG_RELEASE_TIME, "FEG Release Time", "Filter EG", TIME),
        0,
        "How fast the filter moves to the Release Level after you release the key.",
        common_byte(0x1B, (0x00, 0x7F), COMMON_PAGE),
    ),
    row(
        eg_level(FEG_HOLD_LEVEL, "FEG Hold Level"),
        0,
        "Filter offset when you press the key. Positive opens the filter, negative closes it.",
        Mapping::new(COMMON, 0x1C, 2, (0x00, 0xFF), COMMON_PAGE),
    ),
    row(
        eg_level(FEG_ATTACK_LEVEL, "FEG Attack Level"),
        0,
        "Filter offset at the end of the attack.",
        Mapping::new(COMMON, 0x1E, 2, (0x00, 0xFF), COMMON_PAGE),
    ),
    row(
        eg_level(FEG_DECAY_1_LEVEL, "FEG Decay 1 Level"),
        0,
        "Filter offset at the end of the first decay.",
        Mapping::new(COMMON, 0x20, 2, (0x00, 0xFF), COMMON_PAGE),
    ),
    row(
        eg_level(FEG_DECAY_2_LEVEL, "FEG Decay 2 Level"),
        0,
        "Filter offset held while you keep the key down.",
        Mapping::new(COMMON, 0x22, 2, (0x00, 0xFF), COMMON_PAGE),
    ),
    row(
        eg_level(FEG_RELEASE_LEVEL, "FEG Release Level"),
        0,
        "Filter offset reached after you release the key.",
        Mapping::new(COMMON, 0x24, 2, (0x00, 0xFF), COMMON_PAGE),
    ),
    row(
        common(FEG_DEPTH, "FEG Depth", "Filter EG", SIGNED).with_format(ValueFormat::Signed),
        40,
        "How strongly the filter envelope moves the cutoff. 0 turns the envelope off; negative inverts it.",
        common_byte(0x26, (0x00, 0x7F), COMMON_PAGE),
    ),
    row(
        common(FEG_DEPTH_VELOCITY, "FEG Depth Velocity Sensitivity", "Filter EG", SIGNED)
            .with_format(ValueFormat::Signed),
        0,
        "How much playing harder makes the filter envelope stronger (+) or weaker (-).",
        common_byte(0x29, (0x00, 0x7F), COMMON_PAGE),
    ),
    row(
        common(LFO_WAVE, "2nd LFO Wave", "LFO", ValueRange::new(0, 5))
            .with_format(ValueFormat::Choice(LFO_WAVES)),
        0,
        "Shape of the Part's 2nd LFO, the slow wave behind vibrato and tremolo.",
        common_byte(0x47, (0x00, 0x05), LFO_PAGE),
    ),
    row(
        common(LFO_SPEED, "2nd LFO Speed", "LFO", NINETY_NINE),
        30,
        "How fast the 2nd LFO moves.",
        common_byte(0x48, (0x00, 0x63), LFO_PAGE),
    ),
    row(
        common(LFO_DELAY_TIME, "2nd LFO Delay Time", "LFO", NINETY_NINE),
        0,
        "How long after you press a key the 2nd LFO starts.",
        common_byte(0x4A, (0x00, 0x63), LFO_PAGE),
    ),
    row(
        common(LFO_PITCH, "2nd LFO Pitch Modulation", "LFO", NINETY_NINE),
        0,
        "How much the 2nd LFO changes the pitch (vibrato). Each Operator scales it with its own LFO Pitch Sensitivity.",
        common_byte(0x4C, (0x00, 0x63), LFO_PAGE),
    ),
    row(
        common(LFO_AMPLITUDE, "2nd LFO Amplitude Modulation", "LFO", NINETY_NINE),
        0,
        "How much the 2nd LFO changes the volume (tremolo). Each Operator scales it with its own LFO Amplitude Sensitivity.",
        common_byte(0x4D, (0x00, 0x63), LFO_PAGE),
    ),
    row(
        common(LFO_FILTER, "2nd LFO Filter Modulation", "LFO", NINETY_NINE),
        0,
        "How much the 2nd LFO moves the filter (wah).",
        common_byte(0x4E, (0x00, 0x63), LFO_PAGE),
    ),
    row(
        operator(SPECTRAL_FORM, "Spectral Form", "Oscillator", ValueRange::new(0, 6))
            .with_format(ValueFormat::Choice(SPECTRAL_FORMS)),
        0,
        "Basic waveform of this Operator. Sine is the classic FM sound; the others add harmonics.",
        operator_byte(0x09, (0x00, 0x06)),
    ),
    row(
        operator(SPECTRAL_SKIRT, "Spectral Skirt", "Oscillator", EIGHT),
        0,
        "Spreads the harmonics of a non-Sine Spectral Form.",
        operator_byte(0x0A, (0x00, 0x07)),
    ),
    row(
        operator(SPECTRAL_RESONANCE, "Spectral Resonance", "Oscillator", NINETY_NINE),
        0,
        "Moves the resonant peak of the Res Spectral Forms.",
        operator_byte(0x0B, (0x00, 0x63)),
    ),
    row(
        operator(AEG_ATTACK_LEVEL, "AEG Attack Level", "Level", NINETY_NINE),
        99,
        "Level reached at the end of the attack.",
        operator_byte(0x10, (0x00, 0x63)),
    ),
    row(
        operator(AEG_DECAY_1_LEVEL, "AEG Decay 1 Level", "Level", NINETY_NINE),
        99,
        "Level reached at the end of the first decay.",
        operator_byte(0x11, (0x00, 0x63)),
    ),
    row(
        operator(AEG_DECAY_2_LEVEL, "AEG Decay 2 Level", "Level", NINETY_NINE),
        99,
        "Level held while you keep the key down. 0 makes the Operator fade out like a bell or an electric piano tine.",
        operator_byte(0x12, (0x00, 0x63)),
    ),
    row(
        operator(AEG_RELEASE_LEVEL, "AEG Release Level", "Level", NINETY_NINE),
        0,
        "Level reached after you release the key.",
        operator_byte(0x13, (0x00, 0x63)),
    ),
    row(
        operator(AEG_DECAY_2_TIME, "AEG Decay 2 Time", "Level", NINETY_NINE),
        0,
        "How fast the level moves to the Decay 2 Level.",
        operator_byte(0x16, (0x00, 0x63)),
    ),
    row(
        operator(AEG_HOLD_TIME, "AEG Hold Time", "Level", NINETY_NINE),
        0,
        "How long the Operator waits before its attack.",
        operator_byte(0x18, (0x00, 0x63)),
    ),
    row(
        operator(AEG_TIME_KEY_FOLLOW, "AEG Time Key Follow", "Level", EIGHT),
        0,
        "Makes this Operator's envelope faster on higher notes, as on real pianos.",
        operator_byte(0x19, (0x00, 0x07)),
    ),
    row(
        operator(LEVEL_SCALING_LOW_DEPTH, "Level Scaling Low Depth", "Level", NINETY_NINE),
        0,
        "How much this Operator's level changes below the break point.",
        operator_byte(0x1C, (0x00, 0x63)),
    ),
    row(
        operator(LEVEL_SCALING_HIGH_DEPTH, "Level Scaling High Depth", "Level", NINETY_NINE),
        0,
        "How much this Operator's level changes above the break point. Useful to tame harsh high notes.",
        operator_byte(0x1D, (0x00, 0x63)),
    ),
    row(
        operator(OPERATOR_LFO_PITCH, "LFO Pitch Sensitivity", "LFO", EIGHT),
        3,
        "How much the 2nd LFO's pitch modulation affects this Operator.",
        operator_byte(0x21, (0x00, 0x07)),
    ),
    row(
        operator(OPERATOR_LFO_AMPLITUDE, "LFO Amplitude Sensitivity", "LFO", EIGHT),
        3,
        "How much the 2nd LFO's amplitude modulation affects this Operator.",
        operator_byte(0x22, (0x00, 0x07)),
    ),
];

pub(super) fn entries() -> Vec<Entry> {
    let mut entries = part::entries(ENGINE_ID, part::Kind::Fm);
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

    fn refine(&self, parameter: &ParameterDefinition, part: &Part) -> Option<ParameterDefinition> {
        part::refine(parameter, part)
    }

    fn dependents(&self, control: &ParameterDefinition, value: i32) -> Vec<ParameterDefinition> {
        part::dependents(&self.parameters, control, value)
    }

    fn reshapes(&self, parameter: ParameterId) -> bool {
        part::reshapes(parameter)
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
