use super::PART_COUNT;
use super::catalog::{self, Addressing, Block, Entry, Mapping};
use crate::domain::{
    EncodeError, EngineId, EngineProfile, Layout, ParameterChange, ParameterDefinition,
    ParameterId, Scope, ValueFormat, ValueRange,
};
use crate::protocol::SysExMessage;
use crate::protocol::yamaha::DeviceNumber;

pub const ENGINE_ID: EngineId = EngineId::new("modx-plus.effects");

pub const REVERB_TYPE: ParameterId = ParameterId::new("effects.reverb-type");
pub const REVERB_RETURN: ParameterId = ParameterId::new("effects.reverb-return");
pub const REVERB_PAN: ParameterId = ParameterId::new("effects.reverb-pan");
pub const REVERB_SWITCH: ParameterId = ParameterId::new("effects.reverb-switch");
pub const REVERB_PARAMETERS: ParameterId = ParameterId::new("effects.reverb-parameters");
pub const VARIATION_TYPE: ParameterId = ParameterId::new("effects.variation-type");
pub const VARIATION_RETURN: ParameterId = ParameterId::new("effects.variation-return");
pub const VARIATION_PAN: ParameterId = ParameterId::new("effects.variation-pan");
pub const VARIATION_TO_REVERB: ParameterId = ParameterId::new("effects.variation-to-reverb");
pub const VARIATION_SWITCH: ParameterId = ParameterId::new("effects.variation-switch");
pub const VARIATION_PARAMETERS: ParameterId = ParameterId::new("effects.variation-parameters");

pub const EFFECT_TYPE_PAGE: u16 = 144;

const TABLE: &str = "MIDI Parameter Change Table (Performance Common)";
const REVERB: Block = Block::new(0x30, Addressing::Performance { mid: 0x41 }, TABLE);
const VARIATION: Block = Block::new(0x30, Addressing::Performance { mid: 0x42 }, TABLE);

const REVERB_CATEGORY: &str = "Reverb";
const VARIATION_CATEGORY: &str = "Variation";
const OFF_ON: &[&str] = &["Off", "On"];
const PARAMETERS_REASON: &str = "Each effect type has its own parameters, and SynthPilot doesn't know them yet. Change them on the keyboard for now.";

const fn prepend<T: Copy, const N: usize, const M: usize>(first: T, rest: [T; N]) -> [T; M] {
    assert!(M == N + 1, "the result holds one more value");
    let mut out = [first; M];
    let mut index = 0;
    while index < N {
        out[index + 1] = rest[index];
        index += 1;
    }
    out
}

const REVERB_NAMES: [&str; 12] = [
    "HD HALL",
    "REV-X HALL",
    "R3 HALL",
    "SPX HALL",
    "HD ROOM",
    "REV-X ROOM",
    "R3 ROOM",
    "SPX ROOM",
    "HD PLATE",
    "R3 PLATE",
    "SPX STAGE",
    "SPACE SIMULATOR",
];

const REVERB_VALUES: [u32; 12] = [
    0x0083, 0x0080, 0x0081, 0x0082, 0x0093, 0x0090, 0x0091, 0x0092, 0x00A1, 0x00A0, 0x00B0, 0x00C0,
];

const EFFECT_NAMES: [&str; 88] = [
    "SPX HALL",
    "SPX ROOM",
    "SPX STAGE",
    "GATED REVERB",
    "REVERSE REVERB",
    "CROSS DELAY",
    "TEMPO CROSS DELAY",
    "TEMPO DELAY MONO",
    "TEMPO DELAY STEREO",
    "CONTROL DELAY",
    "DELAY LR",
    "DELAY LCR",
    "ANALOG DELAY RETRO",
    "ANALOG DELAY MODERN",
    "G CHORUS",
    "2 MODULATOR",
    "SPX CHORUS",
    "SYMPHONIC",
    "ENSEMBLE DETUNE",
    "VCM FLANGER",
    "CLASSIC FLANGER",
    "TEMPO FLANGER",
    "DYNAMIC FLANGER",
    "CONTROL FLANGER",
    "VCM PHASER MONO",
    "VCM PHASER STEREO",
    "TEMPO PHASER",
    "DYNAMIC PHASER",
    "CONTROL PHASER",
    "AUTO PAN",
    "TREMOLO",
    "ROTARY SPEAKER 1",
    "ROTARY SPEAKER 2",
    "AMP SIMULATOR 1",
    "AMP SIMULATOR 2",
    "COMP DISTORTION",
    "COMP DISTORTION DELAY",
    "US COMBO",
    "JAZZ COMBO",
    "US HIGH GAIN",
    "BRITISH LEAD",
    "MULTI FX",
    "SMALL STEREO",
    "BRITISH COMBO",
    "BRITISH LEGEND",
    "VCM COMPRESSOR 376",
    "CLASSIC COMPRESSOR",
    "MULTI BAND COMP",
    "UNI COMP DOWN",
    "UNI COMP UP",
    "PARALLEL COMP",
    "VCM AUTO WAH",
    "VCM TOUCH WAH",
    "VCM PEDAL WAH",
    "LO-FI",
    "NOISY",
    "DIGITAL TURNTABLE",
    "BIT CRUSHER",
    "RING MODULATOR",
    "DYNAMIC RING MODULATOR",
    "DYNAMIC FILTER",
    "AUTO SYNTH",
    "SPIRALIZER P",
    "TEMPO SPIRALIZER P",
    "SPIRALIZER F",
    "TEMPO SPIRALIZER F",
    "ISOLATOR",
    "SLICE",
    "TECH MODULATION",
    "CONTROL FILTER",
    "VINYL BREAK",
    "BEAT REPEAT EVEN",
    "BEAT REPEAT TRIPLET",
    "BEAT REPEAT EVEN+TRIPLET",
    "BEAT REPEAT FREE",
    "VCM EQ 501",
    "PRESENCE",
    "HARMONIC ENHANCER",
    "STEREOPHONIC OPTIMIZER",
    "TALKING MODULATOR",
    "WAVE FOLDER",
    "VCM MINI FILTER",
    "VCM MINI BOOSTER",
    "DAMPER RESONANCE",
    "PITCH CHANGE",
    "EARLY REFLECTION",
    "VOCODER",
    "NOISE GATE+COMP+EQ",
];

const EFFECT_VALUES: [u32; 88] = [
    0x0082, 0x0092, 0x00B0, 0x00D0, 0x00D8, 0x0100, 0x0110, 0x0120, 0x0128, 0x0130, 0x0140, 0x0150,
    0x0160, 0x0168, 0x0180, 0x0190, 0x01A0, 0x01B0, 0x01C0, 0x0200, 0x0210, 0x0220, 0x0230, 0x0208,
    0x0280, 0x0290, 0x02A0, 0x02B0, 0x0298, 0x0300, 0x0310, 0x0320, 0x0330, 0x0380, 0x0390, 0x03A0,
    0x03B0, 0x03C0, 0x03C1, 0x03C2, 0x03C3, 0x03C4, 0x03C5, 0x03C6, 0x03C7, 0x0400, 0x0410, 0x0420,
    0x0430, 0x0438, 0x0440, 0x0500, 0x0510, 0x0520, 0x0580, 0x0590, 0x05A0, 0x05B0, 0x0600, 0x0610,
    0x0620, 0x0630, 0x0638, 0x0639, 0x063A, 0x063B, 0x0640, 0x0650, 0x0660, 0x0628, 0x0670, 0x0678,
    0x0679, 0x067A, 0x067B, 0x0680, 0x0688, 0x0690, 0x0698, 0x06A0, 0x06A8, 0x06A9, 0x06AA, 0x06B0,
    0x06C0, 0x06D0, 0x06E0, 0x06F0,
];

const REVERB_LABELS: [&str; 13] = prepend("NO EFFECT", REVERB_NAMES);
const REVERB_TABLE: [u32; 13] = prepend(0x0000, REVERB_VALUES);
const VARIATION_LABELS: [&str; 89] = prepend("NO EFFECT", EFFECT_NAMES);
pub(super) const INSERTION_LABELS: [&str; 89] = prepend("THRU", EFFECT_NAMES);
pub(super) const EFFECT_TABLE: [u32; 89] = prepend(0x0000, EFFECT_VALUES);

pub(super) const fn effect_type(
    id: ParameterId,
    name: &'static str,
    engine: EngineId,
    category: &'static str,
    scope: Scope,
    labels: &'static [&'static str],
) -> ParameterDefinition {
    assert!(
        !labels.is_empty(),
        "an effect type needs at least one label"
    );
    let mut last = 0;
    let mut index = 1;
    while index < labels.len() {
        last += 1;
        index += 1;
    }
    ParameterDefinition::new(id, name, engine, category, scope, ValueRange::new(0, last))
        .with_format(ValueFormat::Choice(labels))
        .with_default(0)
}

const fn common(
    id: ParameterId,
    name: &'static str,
    category: &'static str,
    range: ValueRange,
) -> ParameterDefinition {
    ParameterDefinition::new(id, name, ENGINE_ID, category, Scope::Performance, range)
}

const ENTRIES: &[Entry] = &[
    Entry::verified(
        effect_type(
            REVERB_TYPE,
            "Reverb Type",
            ENGINE_ID,
            REVERB_CATEGORY,
            Scope::Performance,
            &REVERB_LABELS,
        )
        .with_default(2)
        .with_help("Kind of reverb shared by every Part. NO EFFECT turns it off."),
        Mapping::listed(REVERB, 0x00, 2, &REVERB_TABLE, 197),
    ),
    Entry::verified(
        common(
            REVERB_RETURN,
            "Reverb Return",
            REVERB_CATEGORY,
            ValueRange::new(0, 127),
        )
        .with_default(64)
        .with_help("How loud the reverb is in the mix."),
        Mapping::new(REVERB, 0x44, 1, (0x00, 0x7F), 198),
    ),
    Entry::verified(
        common(
            REVERB_PAN,
            "Reverb Pan",
            REVERB_CATEGORY,
            ValueRange::new(-63, 63),
        )
        .with_format(ValueFormat::Pan)
        .with_default(0)
        .with_help("Where the reverb sits in the stereo field, from L63 (left) through C (center) to R63 (right)."),
        Mapping::new(REVERB, 0x45, 1, (0x01, 0x7F), 198),
    ),
    Entry::verified(
        common(
            REVERB_SWITCH,
            "Reverb Switch",
            REVERB_CATEGORY,
            ValueRange::new(0, 1),
        )
        .with_format(ValueFormat::Choice(OFF_ON))
        .with_default(1)
        .with_help("Turns the reverb on or off for the whole Performance."),
        Mapping::new(REVERB, 0x46, 1, (0x00, 0x01), 198),
    ),
    Entry::unverified(
        common(
            REVERB_PARAMETERS,
            "Reverb Parameters 1–24",
            REVERB_CATEGORY,
            ValueRange::new(0, 0x3FFF),
        ),
        PARAMETERS_REASON,
    ),
    Entry::verified(
        effect_type(
            VARIATION_TYPE,
            "Variation Type",
            ENGINE_ID,
            VARIATION_CATEGORY,
            Scope::Performance,
            &VARIATION_LABELS,
        )
        .with_default(15)
        .with_help("Second shared effect, such as a chorus, a delay or another reverb. NO EFFECT turns it off."),
        Mapping::listed(VARIATION, 0x00, 2, &EFFECT_TABLE, 198),
    ),
    Entry::verified(
        common(
            VARIATION_RETURN,
            "Variation Return",
            VARIATION_CATEGORY,
            ValueRange::new(0, 127),
        )
        .with_default(96)
        .with_help("How loud the Variation effect is in the mix."),
        Mapping::new(VARIATION, 0x44, 1, (0x00, 0x7F), 198),
    ),
    Entry::verified(
        common(
            VARIATION_PAN,
            "Variation Pan",
            VARIATION_CATEGORY,
            ValueRange::new(-63, 63),
        )
        .with_format(ValueFormat::Pan)
        .with_default(0)
        .with_help("Where the Variation effect sits in the stereo field, from L63 (left) through C (center) to R63 (right)."),
        Mapping::new(VARIATION, 0x45, 1, (0x01, 0x7F), 198),
    ),
    Entry::verified(
        common(
            VARIATION_TO_REVERB,
            "Send Variation To Reverb",
            VARIATION_CATEGORY,
            ValueRange::new(0, 127),
        )
        .with_default(0)
        .with_help("Sends the Variation effect into the Reverb, for example to give a delay some space."),
        Mapping::new(VARIATION, 0x46, 1, (0x00, 0x7F), 198),
    ),
    Entry::verified(
        common(
            VARIATION_SWITCH,
            "Variation Switch",
            VARIATION_CATEGORY,
            ValueRange::new(0, 1),
        )
        .with_format(ValueFormat::Choice(OFF_ON))
        .with_default(1)
        .with_help("Turns the Variation effect on or off for the whole Performance."),
        Mapping::new(VARIATION, 0x47, 1, (0x00, 0x01), 198),
    ),
    Entry::unverified(
        common(
            VARIATION_PARAMETERS,
            "Variation Parameters 1–24",
            VARIATION_CATEGORY,
            ValueRange::new(0, 0x3FFF),
        ),
        PARAMETERS_REASON,
    ),
];

pub(super) fn entries() -> Vec<Entry> {
    ENTRIES.to_vec()
}

pub struct Effects {
    device_number: DeviceNumber,
    parameters: Vec<ParameterDefinition>,
}

impl Effects {
    pub fn new(device_number: DeviceNumber) -> Self {
        Self {
            device_number,
            parameters: catalog::definitions(ENTRIES),
        }
    }
}

impl EngineProfile for Effects {
    fn id(&self) -> EngineId {
        ENGINE_ID
    }

    fn name(&self) -> &'static str {
        "Effects"
    }

    fn description(&self) -> &'static str {
        "Reverb and Variation are shared by every Part. Each Part feeds them with its Reverb Send and Variation Send."
    }

    fn element_label(&self) -> &'static str {
        ""
    }

    fn layout(&self) -> Layout {
        Layout {
            parts: PART_COUNT,
            elements: 0,
        }
    }

    fn parameters(&self) -> &[ParameterDefinition] {
        &self.parameters
    }

    fn encode(&self, change: &ParameterChange) -> Result<SysExMessage, EncodeError> {
        catalog::encode(ENTRIES, self.layout(), self.device_number, change)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Location, Support};

    fn engine() -> Effects {
        Effects::new(DeviceNumber::default())
    }

    fn hex(id: ParameterId, value: i32) -> String {
        engine()
            .encode(&ParameterChange::new(id, Location::performance(), value))
            .unwrap()
            .to_hex()
    }

    #[test]
    fn effect_type_tables_match_the_data_list() {
        assert_eq!(REVERB_LABELS[2], "REV-X HALL");
        assert_eq!(REVERB_TABLE[2], 0x0080);
        assert_eq!(REVERB_LABELS[12], "SPACE SIMULATOR");
        assert_eq!(REVERB_TABLE[12], 0x00C0);
        assert_eq!(VARIATION_LABELS[15], "G CHORUS");
        assert_eq!(EFFECT_TABLE[15], 0x0180);
        assert_eq!(INSERTION_LABELS[0], "THRU");
        assert_eq!(INSERTION_LABELS[88], "NOISE GATE+COMP+EQ");
        assert_eq!(EFFECT_TABLE[88], (0x0D << 7) | 0x70);
    }

    #[test]
    fn reverb_and_variation_types_encode_as_msb_and_lsb() {
        assert_eq!(hex(REVERB_TYPE, 2), "F0 43 10 7F 1C 07 30 41 00 01 00 F7");
        assert_eq!(hex(REVERB_TYPE, 1), "F0 43 10 7F 1C 07 30 41 00 01 03 F7");
        assert_eq!(
            hex(VARIATION_TYPE, 15),
            "F0 43 10 7F 1C 07 30 42 00 03 00 F7"
        );
        assert_eq!(
            hex(VARIATION_TYPE, 0),
            "F0 43 10 7F 1C 07 30 42 00 00 00 F7"
        );
    }

    #[test]
    fn returns_and_sends_use_the_common_blocks() {
        assert_eq!(hex(REVERB_RETURN, 64), "F0 43 10 7F 1C 07 30 41 44 40 F7");
        assert_eq!(hex(REVERB_PAN, -63), "F0 43 10 7F 1C 07 30 41 45 01 F7");
        assert_eq!(hex(REVERB_SWITCH, 0), "F0 43 10 7F 1C 07 30 41 46 00 F7");
        assert_eq!(
            hex(VARIATION_RETURN, 96),
            "F0 43 10 7F 1C 07 30 42 44 60 F7"
        );
        assert_eq!(hex(VARIATION_PAN, 63), "F0 43 10 7F 1C 07 30 42 45 7F F7");
        assert_eq!(
            hex(VARIATION_TO_REVERB, 20),
            "F0 43 10 7F 1C 07 30 42 46 14 F7"
        );
        assert_eq!(hex(VARIATION_SWITCH, 1), "F0 43 10 7F 1C 07 30 42 47 01 F7");
    }

    #[test]
    fn effects_describe_themselves() {
        let effects = engine();
        assert_eq!(effects.id(), ENGINE_ID);
        assert_eq!(effects.name(), "Effects");
        assert!(effects.description().contains("Reverb Send"));
        assert_eq!(effects.element_label(), "");
        assert_eq!(effects.layout().elements, 0);
        assert!(
            effects
                .parameters()
                .iter()
                .all(|parameter| parameter.scope == Scope::Performance)
        );
        let parameters = effects.parameter(REVERB_PARAMETERS).unwrap();
        assert!(matches!(parameters.support, Support::NotImplemented { .. }));
        assert_eq!(entries().len(), effects.parameters().len());
    }

    #[test]
    fn performance_parameters_reject_part_locations() {
        let change = ParameterChange::new(REVERB_RETURN, Location::part(1), 10);
        assert!(engine().encode(&change).is_err());
    }

    #[test]
    fn builders_run_at_runtime_too() {
        let labels: [&str; 3] = prepend("first", ["second", "third"]);
        assert_eq!(labels, ["first", "second", "third"]);
        let definition = common(
            REVERB_RETURN,
            "Return",
            REVERB_CATEGORY,
            ValueRange::new(0, 1),
        );
        assert_eq!(definition.scope, Scope::Performance);
        let types = effect_type(
            VARIATION_TYPE,
            "Type",
            ENGINE_ID,
            VARIATION_CATEGORY,
            Scope::Performance,
            &REVERB_LABELS,
        );
        assert_eq!(types.range, ValueRange::new(0, 12));
    }
}
