use super::PART_COUNT;
use super::catalog::{self, Addressing, Block, Entry, Mapping};
use super::part;
use super::waves;
use crate::domain::{
    EncodeError, EngineId, EngineProfile, Layout, Location, ParameterChange, ParameterDefinition,
    ParameterId, Part, Scope, Unit, ValueFormat, ValueRange,
};
use crate::protocol::SysExMessage;
use crate::protocol::yamaha::DeviceNumber;

pub const ENGINE_ID: EngineId = EngineId::new("awm2");
pub const ELEMENT_COUNT: u8 = 8;

pub const ELEMENT_SWITCH: ParameterId = ParameterId::new("awm2.element-switch");
pub const XA_CONTROL: ParameterId = ParameterId::new("awm2.xa-control");
pub const WAVE_NUMBER: ParameterId = ParameterId::new("awm2.wave-number");
pub const ELEMENT_LEVEL: ParameterId = ParameterId::new("awm2.element-level");
pub const ELEMENT_CONNECTION: ParameterId = ParameterId::new("awm2.element-connection");
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

pub const NOTE_LIMIT_LOW: ParameterId = ParameterId::new("awm2.note-limit-low");
pub const NOTE_LIMIT_HIGH: ParameterId = ParameterId::new("awm2.note-limit-high");
pub const VELOCITY_LIMIT_LOW: ParameterId = ParameterId::new("awm2.velocity-limit-low");
pub const VELOCITY_LIMIT_HIGH: ParameterId = ParameterId::new("awm2.velocity-limit-high");
pub const VELOCITY_CROSS_FADE: ParameterId = ParameterId::new("awm2.velocity-cross-fade");
pub const AEG_TIME_VELOCITY_SEGMENT: ParameterId =
    ParameterId::new("awm2.aeg-time-velocity-segment");
pub const AEG_TIME_VELOCITY_SENSITIVITY: ParameterId =
    ParameterId::new("awm2.aeg-time-velocity-sensitivity");
pub const AEG_TIME_KEY_FOLLOW: ParameterId = ParameterId::new("awm2.aeg-time-key-follow");
pub const WAVE_SELECT: ParameterId = ParameterId::new("awm2.wave-select");
pub const AEG_INITIAL_LEVEL: ParameterId = ParameterId::new("awm2.aeg-initial-level");
pub const AEG_ATTACK_LEVEL: ParameterId = ParameterId::new("awm2.aeg-attack-level");
pub const AEG_DECAY_1_LEVEL: ParameterId = ParameterId::new("awm2.aeg-decay-1-level");
pub const AEG_DECAY_2_LEVEL: ParameterId = ParameterId::new("awm2.aeg-decay-2-level");
pub const PITCH_VELOCITY_SENSITIVITY: ParameterId =
    ParameterId::new("awm2.pitch-velocity-sensitivity");
pub const FILTER_CUTOFF_VELOCITY: ParameterId = ParameterId::new("awm2.filter-cutoff-velocity");
pub const FEG_HOLD_TIME: ParameterId = ParameterId::new("awm2.feg-hold-time");
pub const FEG_ATTACK_TIME: ParameterId = ParameterId::new("awm2.feg-attack-time");
pub const FEG_DECAY_1_TIME: ParameterId = ParameterId::new("awm2.feg-decay-1-time");
pub const FEG_DECAY_2_TIME: ParameterId = ParameterId::new("awm2.feg-decay-2-time");
pub const FEG_RELEASE_TIME: ParameterId = ParameterId::new("awm2.feg-release-time");
pub const FEG_HOLD_LEVEL: ParameterId = ParameterId::new("awm2.feg-hold-level");
pub const FEG_ATTACK_LEVEL: ParameterId = ParameterId::new("awm2.feg-attack-level");
pub const FEG_DECAY_1_LEVEL: ParameterId = ParameterId::new("awm2.feg-decay-1-level");
pub const FEG_DECAY_2_LEVEL: ParameterId = ParameterId::new("awm2.feg-decay-2-level");
pub const FEG_RELEASE_LEVEL: ParameterId = ParameterId::new("awm2.feg-release-level");
pub const FEG_DEPTH: ParameterId = ParameterId::new("awm2.feg-depth");
pub const FEG_DEPTH_VELOCITY: ParameterId = ParameterId::new("awm2.feg-depth-velocity");
pub const LFO_WAVE: ParameterId = ParameterId::new("awm2.lfo-wave");
pub const LFO_DELAY_TIME: ParameterId = ParameterId::new("awm2.lfo-delay-time");
pub const LFO_SPEED: ParameterId = ParameterId::new("awm2.lfo-speed");
pub const LFO_AMPLITUDE: ParameterId = ParameterId::new("awm2.lfo-amplitude");
pub const LFO_PITCH: ParameterId = ParameterId::new("awm2.lfo-pitch");
pub const LFO_FILTER: ParameterId = ParameterId::new("awm2.lfo-filter");
pub const LFO_FADE_IN_TIME: ParameterId = ParameterId::new("awm2.lfo-fade-in-time");

const TABLE: &str = "MIDI Parameter Change Table (Normal Part Element)";
const GROUP_1: Block = Block::new(0x41, Addressing::Element, TABLE);
const GROUP_2: Block = Block::new(0x42, Addressing::Element, TABLE);
const GROUP_1_PAGE: u16 = 217;
const GROUP_2_PAGE: u16 = 218;

const OFF_ON: &[&str] = &["Off", "On"];
const WAVE_BANKS: &[&str] = &[
    "Preset",
    "User",
    "Library 1",
    "Library 2",
    "Library 3",
    "Library 4",
    "Library 5",
    "Library 6",
    "Library 7",
    "Library 8",
];
const LFO_WAVES: &[&str] = &["Saw", "Triangle", "Square"];
const TIME_SEGMENTS: &[&str] = &["Attack", "Atk+Dcy", "Decay", "Atk+Rls", "All"];
const VELOCITY: ValueRange = ValueRange::new(1, 127);
const SIGNED: ValueRange = ValueRange::new(-64, 63);
const LEVEL: ValueRange = ValueRange::new(0, 127);
const EG_LEVEL: ValueRange = ValueRange::new(-128, 127);
const CONNECTIONS: &[&str] = &["Thru", "InsA", "InsB"];
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

const fn row(
    definition: ParameterDefinition,
    default: i32,
    help: &'static str,
    mapping: Mapping,
) -> Entry {
    Entry::verified(definition.with_default(default).with_help(help), mapping)
}

const fn signed(
    id: ParameterId,
    name: &'static str,
    category: &'static str,
) -> ParameterDefinition {
    element(id, name, category, SIGNED).with_format(ValueFormat::Signed)
}

const fn eg_level(id: ParameterId, name: &'static str) -> ParameterDefinition {
    element(id, name, "Filter EG", EG_LEVEL).with_format(ValueFormat::Signed)
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
    Entry::verified(
        element(
            ELEMENT_CONNECTION,
            "Element Connection Switch",
            "Effects",
            ValueRange::new(0, 2),
        )
        .with_format(ValueFormat::Choice(CONNECTIONS))
        .with_default(1)
        .with_help("Sends this Element through Insertion A, Insertion B, or past both (Thru)."),
        group_1(0x17, (0x00, 0x02)),
    ),
    row(
        element(WAVE_SELECT, "Wave Bank", "Oscillator", ValueRange::new(0, 9))
            .with_format(ValueFormat::Choice(WAVE_BANKS)),
        0,
        "Where this Element's Wave comes from. Wave names in SynthPilot are the Preset Waves.",
        group_1(0x01, (0x00, 0x09)),
    ),
    row(
        element(NOTE_LIMIT_LOW, "Note Limit Low", "Key and Velocity Range", LEVEL)
            .with_format(ValueFormat::Note),
        0,
        "Lowest note this Element plays. Use it to split the keyboard between Elements.",
        group_1(0x0D, (0x00, 0x7F)),
    ),
    row(
        element(NOTE_LIMIT_HIGH, "Note Limit High", "Key and Velocity Range", LEVEL)
            .with_format(ValueFormat::Note),
        127,
        "Highest note this Element plays.",
        group_1(0x0E, (0x00, 0x7F)),
    ),
    row(
        element(VELOCITY_LIMIT_LOW, "Velocity Limit Low", "Key and Velocity Range", VELOCITY),
        1,
        "Softest playing that triggers this Element. Layered pianos give each Element its own velocity range, so soft and hard playing pick different samples.",
        group_1(0x0F, (0x01, 0x7F)),
    ),
    row(
        element(VELOCITY_LIMIT_HIGH, "Velocity Limit High", "Key and Velocity Range", VELOCITY),
        127,
        "Hardest playing that still triggers this Element.",
        group_1(0x10, (0x01, 0x7F)),
    ),
    row(
        element(VELOCITY_CROSS_FADE, "Velocity Cross Fade", "Key and Velocity Range", LEVEL),
        0,
        "Fades this Element in and out near its velocity limits instead of switching abruptly.",
        group_1(0x11, (0x00, 0x7F)),
    ),
    row(
        element(
            AEG_TIME_VELOCITY_SEGMENT,
            "AEG Time Velocity Segment",
            "Amplitude",
            ValueRange::new(0, 4),
        )
        .with_format(ValueFormat::Choice(TIME_SEGMENTS)),
        4,
        "Which parts of the volume envelope get faster or slower with velocity.",
        group_1(0x37, (0x00, 0x04)),
    ),
    row(
        signed(AEG_TIME_VELOCITY_SENSITIVITY, "AEG Time Velocity Sensitivity", "Amplitude"),
        0,
        "Positive makes the envelope faster when you play harder, for a sharper attack.",
        group_1(0x38, (0x00, 0x7F)),
    ),
    row(
        signed(AEG_TIME_KEY_FOLLOW, "AEG Time Key Follow", "Amplitude"),
        0,
        "Positive makes the envelope faster on higher notes, as on real pianos.",
        group_1(0x39, (0x00, 0x7F)),
    ),
    row(
        element(AEG_INITIAL_LEVEL, "AEG Initial Level", "Amplitude", LEVEL),
        0,
        "Volume at the moment you press the key, before the attack.",
        group_1(0x32, (0x00, 0x7F)),
    ),
    row(
        element(AEG_ATTACK_LEVEL, "AEG Attack Level", "Amplitude", LEVEL),
        127,
        "Volume reached at the end of the attack.",
        group_1(0x33, (0x00, 0x7F)),
    ),
    row(
        element(AEG_DECAY_1_LEVEL, "AEG Decay 1 Level", "Amplitude", LEVEL),
        127,
        "Volume reached at the end of the first decay.",
        group_1(0x34, (0x00, 0x7F)),
    ),
    row(
        element(AEG_DECAY_2_LEVEL, "AEG Decay 2 Level", "Amplitude", LEVEL),
        127,
        "Volume held while you keep the key down (sustain). 0 makes notes fade out like a piano or a bell.",
        group_1(0x35, (0x00, 0x7F)),
    ),
    row(
        signed(PITCH_VELOCITY_SENSITIVITY, "Pitch Velocity Sensitivity", "Pitch"),
        0,
        "How much playing harder raises (+) or lowers (-) the pitch of this Element.",
        group_1(0x4B, (0x00, 0x7F)),
    ),
    row(
        signed(FILTER_CUTOFF_VELOCITY, "Filter Cutoff Velocity Sensitivity", "Filter"),
        0,
        "How much playing harder makes this Element brighter (+) or darker (-). Electric pianos use a positive value.",
        group_2(0x03, 1, (0x00, 0x7F)),
    ),
    row(
        element(FEG_HOLD_TIME, "FEG Hold Time", "Filter EG", LEVEL),
        0,
        "How long the filter stays at its starting point before the attack.",
        group_2(0x0E, 1, (0x00, 0x7F)),
    ),
    row(
        element(FEG_ATTACK_TIME, "FEG Attack Time", "Filter EG", LEVEL),
        0,
        "How fast the filter moves to the Attack Level after you press a key.",
        group_2(0x0F, 1, (0x00, 0x7F)),
    ),
    row(
        element(FEG_DECAY_1_TIME, "FEG Decay 1 Time", "Filter EG", LEVEL),
        64,
        "How fast the filter moves from the Attack Level to the Decay 1 Level.",
        group_2(0x10, 1, (0x00, 0x7F)),
    ),
    row(
        element(FEG_DECAY_2_TIME, "FEG Decay 2 Time", "Filter EG", LEVEL),
        64,
        "How fast the filter moves to the Decay 2 Level, where it stays while you hold the key.",
        group_2(0x11, 1, (0x00, 0x7F)),
    ),
    row(
        element(FEG_RELEASE_TIME, "FEG Release Time", "Filter EG", LEVEL),
        80,
        "How fast the filter moves to the Release Level after you release the key.",
        group_2(0x12, 1, (0x00, 0x7F)),
    ),
    row(
        eg_level(FEG_HOLD_LEVEL, "FEG Hold Level"),
        0,
        "Filter offset when you press the key. Positive opens the filter, negative closes it.",
        group_2(0x13, 2, (0x00, 0xFF)),
    ),
    row(
        eg_level(FEG_ATTACK_LEVEL, "FEG Attack Level"),
        127,
        "Filter offset at the end of the attack. A high value with a short decay gives a bright, percussive start.",
        group_2(0x15, 2, (0x00, 0xFF)),
    ),
    row(
        eg_level(FEG_DECAY_1_LEVEL, "FEG Decay 1 Level"),
        127,
        "Filter offset at the end of the first decay.",
        group_2(0x17, 2, (0x00, 0xFF)),
    ),
    row(
        eg_level(FEG_DECAY_2_LEVEL, "FEG Decay 2 Level"),
        127,
        "Filter offset held while you keep the key down.",
        group_2(0x19, 2, (0x00, 0xFF)),
    ),
    row(
        eg_level(FEG_RELEASE_LEVEL, "FEG Release Level"),
        0,
        "Filter offset reached after you release the key.",
        group_2(0x1B, 2, (0x00, 0xFF)),
    ),
    row(
        signed(FEG_DEPTH, "FEG Depth", "Filter EG"),
        40,
        "How strongly the filter envelope moves the cutoff. 0 turns the envelope off; negative inverts it.",
        group_2(0x1D, 1, (0x00, 0x7F)),
    ),
    row(
        signed(FEG_DEPTH_VELOCITY, "FEG Depth Velocity Sensitivity", "Filter EG"),
        0,
        "How much playing harder makes the filter envelope stronger (+) or weaker (-).",
        group_2(0x20, 1, (0x00, 0x7F)),
    ),
    row(
        element(LFO_WAVE, "LFO Wave", "LFO", ValueRange::new(0, 2))
            .with_format(ValueFormat::Choice(LFO_WAVES)),
        1,
        "Shape of this Element's LFO, the slow wave that creates vibrato, tremolo or wah.",
        group_2(0x3A, 1, (0x00, 0x02)),
    ),
    row(
        element(LFO_DELAY_TIME, "LFO Delay Time", "LFO", LEVEL),
        0,
        "How long after you press a key the LFO starts.",
        group_2(0x3C, 1, (0x00, 0x7F)),
    ),
    row(
        element(LFO_SPEED, "LFO Speed", "LFO", ValueRange::new(0, 63)),
        38,
        "How fast the LFO moves.",
        group_2(0x3D, 1, (0x00, 0x3F)),
    ),
    row(
        element(LFO_AMPLITUDE, "LFO Amplitude Modulation", "LFO", LEVEL),
        0,
        "How much the LFO changes this Element's volume (tremolo). Classic electric pianos use some.",
        group_2(0x3E, 1, (0x00, 0x7F)),
    ),
    row(
        element(LFO_PITCH, "LFO Pitch Modulation", "LFO", LEVEL),
        0,
        "How much the LFO changes this Element's pitch (vibrato).",
        group_2(0x3F, 1, (0x00, 0x7F)),
    ),
    row(
        element(LFO_FILTER, "LFO Filter Modulation", "LFO", LEVEL),
        0,
        "How much the LFO moves this Element's filter (wah).",
        group_2(0x40, 1, (0x00, 0x7F)),
    ),
    row(
        element(LFO_FADE_IN_TIME, "LFO Fade In Time", "LFO", LEVEL),
        0,
        "How long the LFO takes to reach full depth after it starts.",
        group_2(0x41, 1, (0x00, 0x7F)),
    ),
];

pub(super) fn wave_number(engine: EngineId, scope: Scope, mapping: Mapping) -> Entry {
    Entry::verified(
        ParameterDefinition::new(
            WAVE_NUMBER,
            "Wave",
            engine,
            "Oscillator",
            scope,
            ValueRange::new(1, waves::COUNT_I32),
        )
        .with_format(ValueFormat::Listed {
            first: 1,
            values: waves::all(),
        })
        .with_default(1)
        .with_help("The recorded sound this Element plays, by number and name from the Preset Waves. This choice shapes the sound more than anything else."),
        mapping,
    )
}

pub(super) fn entries() -> Vec<Entry> {
    let mut entries = part::entries(ENGINE_ID, part::Kind::Normal);
    entries.push(wave_number(
        ENGINE_ID,
        Scope::Element,
        Mapping::new(GROUP_1, 0x03, 2, (1, waves::COUNT_U32), GROUP_1_PAGE),
    ));
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
