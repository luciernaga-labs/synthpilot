//! Part settings shown on the MODX+ Part Common screens, checked byte for byte
//! against the MIDI Parameter Change Table (Performance Part), pages 207–210 of
//! the MODX+ Data List.

use synthpilot_core::devices::modx_plus::{Awm2, Drum, FmX, part};
use synthpilot_core::domain::{EngineProfile, Location, ParameterChange, ParameterId, Support};
use synthpilot_core::protocol::yamaha::DeviceNumber;

fn encode(engine: &dyn EngineProfile, id: ParameterId, location: Location, value: i32) -> String {
    engine
        .encode(&ParameterChange::new(id, location, value))
        .unwrap()
        .to_hex()
}

#[test]
fn part_common_settings_match_the_performance_part_table() {
    let awm2 = Awm2::new(DeviceNumber::default());
    let part = Location::part(1);
    let cases = [
        (part::MAIN_CATEGORY, 9, "31 00 14 09"),
        (part::MAIN_CATEGORY, 16, "31 00 14 10"),
        (part::VELOCITY_LIMIT_LOW, 1, "31 00 1C 01"),
        (part::VELOCITY_LIMIT_HIGH, 127, "31 00 1D 7F"),
        (part::NOTE_LIMIT_LOW, 0, "31 00 1E 00"),
        (part::NOTE_LIMIT_HIGH, 127, "31 00 1F 7F"),
        (part::PITCH_BEND_UPPER, 2, "31 00 20 42"),
        (part::PITCH_BEND_LOWER, -2, "31 00 21 3E"),
        (part::PITCH_BEND_LOWER, -48, "31 00 21 10"),
        (part::PITCH_BEND_UPPER, 24, "31 00 20 58"),
        (part::PORTAMENTO_SWITCH, 1, "31 00 31 01"),
        (part::PORTAMENTO_TIME, 64, "31 00 32 40"),
        (part::PORTAMENTO_MODE, 0, "31 00 33 00"),
        (part::KEY_ASSIGN, 1, "31 00 35 01"),
        (part::MONO_POLY, 0, "31 00 36 00"),
        (part::PART_OUTPUT, 0, "31 00 3F 00"),
        (part::PART_OUTPUT, 1, "31 00 3F 09"),
        (part::PART_OUTPUT, 5, "31 00 3F 42"),
        (part::PART_OUTPUT, 13, "31 00 3F 7D"),
        (part::ELEMENT_PAN_SWITCH, 0, "31 00 45 00"),
        (part::ARP_PLAY_ONLY, 1, "31 60 03 01"),
        (part::PART_SWITCH, 0, "31 00 16 00"),
        (part::KEYBOARD_CONTROL, 1, "31 00 17 01"),
        (part::MUTE, 1, "31 00 18 01"),
    ];
    for (id, value, bytes) in cases {
        assert_eq!(
            encode(&awm2, id, part, value),
            format!("F0 43 10 7F 1C 07 {bytes} F7"),
            "{id} = {value}"
        );
    }
    assert_eq!(
        encode(&awm2, part::ARP_PLAY_ONLY, Location::part(16), 0),
        "F0 43 10 7F 1C 07 31 6F 03 00 F7"
    );
}

#[test]
fn defaults_match_an_initialized_part() {
    let awm2 = Awm2::new(DeviceNumber::default());
    let check = |id, value: &str| {
        let definition = awm2.parameter(id).unwrap();
        assert_eq!(
            definition.display_value(definition.default.unwrap()),
            value,
            "{id}"
        );
    };
    check(part::MAIN_CATEGORY, "No Assign");
    check(part::MONO_POLY, "Poly");
    check(part::KEY_ASSIGN, "Multi");
    check(part::ELEMENT_PAN_SWITCH, "On");
    check(part::ARP_PLAY_ONLY, "Off");
    check(part::PART_OUTPUT, "MainL&R");
    check(part::VELOCITY_LIMIT_LOW, "1");
    check(part::VELOCITY_LIMIT_HIGH, "127");
    check(part::NOTE_LIMIT_LOW, "C-2");
    check(part::NOTE_LIMIT_HIGH, "G8");
    check(part::PITCH_BEND_UPPER, "+2 st");
    check(part::PITCH_BEND_LOWER, "-2 st");
    check(part::PORTAMENTO_MODE, "Full-time");
}

#[test]
fn categories_name_bells_and_pads() {
    let awm2 = Awm2::new(DeviceNumber::default());
    let category = awm2.parameter(part::MAIN_CATEGORY).unwrap();
    assert_eq!(category.value_named("Chromatic Perc"), Some(11));
    assert_eq!(category.value_named("Pad/Choir"), Some(9));
    assert_eq!(category.value_named("Piano"), Some(0));
    let sub = awm2.parameter(part::SUB_CATEGORY).unwrap();
    assert!(matches!(sub.support, Support::NotImplemented { .. }));
}

#[test]
fn only_drum_parts_offer_the_drum_output() {
    let awm2 = Awm2::new(DeviceNumber::default());
    let fm = FmX::new(DeviceNumber::default());
    let drum = Drum::new(DeviceNumber::default());
    assert_eq!(awm2.parameter(part::PART_OUTPUT).unwrap().range.max(), 13);
    assert_eq!(fm.parameter(part::PART_OUTPUT).unwrap().range.max(), 13);
    let output = drum.parameter(part::PART_OUTPUT).unwrap();
    assert_eq!(output.display_value(14), "Drum");
    assert_eq!(
        encode(&drum, part::PART_OUTPUT, Location::part(10), 14),
        "F0 43 10 7F 1C 07 31 09 3F 7F F7"
    );
    assert!(drum.parameter(part::MONO_POLY).is_none());
    assert!(drum.parameter(part::PORTAMENTO_SWITCH).is_none());
}

#[test]
fn insertion_b_offers_only_its_column_of_the_effect_type_list() {
    let awm2 = Awm2::new(DeviceNumber::default());
    let a = awm2.parameter(part::INSERTION_A_TYPE).unwrap();
    let b = awm2.parameter(part::INSERTION_B_TYPE).unwrap();
    assert_eq!(a.range.max(), 88);
    assert_eq!(b.range.max(), 83);
    assert_eq!(a.value_named("BEAT REPEAT FREE"), Some(75));
    assert_eq!(b.value_named("BEAT REPEAT FREE"), None);
    assert_eq!(b.value_named("VCM EQ 501"), Some(71));
    assert_eq!(
        encode(&awm2, part::INSERTION_B_TYPE, Location::part(1), 71),
        "F0 43 10 7F 1C 07 31 30 00 0D 00 F7"
    );
    assert_eq!(
        encode(&awm2, part::INSERTION_A_TYPE, Location::part(1), 71),
        "F0 43 10 7F 1C 07 31 20 00 0C 70 F7"
    );
    assert_eq!(a.group_of(32), Some("Tremolo & Rotary"));
    assert_eq!(a.group_starting_at(6), Some("Delay"));
    assert_eq!(b.group_of(83), Some("Misc"));
    assert_eq!(a.group_of(0), None);
}

fn keyboard_change(mid: u8, low: u8, data: &[u8]) -> Vec<u8> {
    use synthpilot_core::protocol::yamaha::{self, Address, ModelId};
    yamaha::parameter_change(
        DeviceNumber::default(),
        ModelId::new(&[0x7F, 0x1C, 0x07]),
        Address::new(0x31, mid, low).unwrap(),
        data,
    )
    .unwrap()
    .into_bytes()
}

#[test]
fn keyboard_edits_to_part_settings_are_decoded() {
    use synthpilot_core::devices::modx_plus::ModxPlus;
    use synthpilot_core::domain::{DeviceProfile, Received};
    let device = ModxPlus::new();
    let cases = [
        (0x02, 0x14, vec![0x0B], part::MAIN_CATEGORY, 3, 11),
        (0x00, 0x36, vec![0x00], part::MONO_POLY, 1, 0),
        (0x00, 0x3F, vec![0x0A], part::PART_OUTPUT, 1, 2),
        (0x62, 0x03, vec![0x01], part::ARP_PLAY_ONLY, 3, 1),
        (0x30, 0x00, vec![0x0D, 0x00], part::INSERTION_B_TYPE, 1, 71),
    ];
    for (mid, low, data, id, number, value) in cases {
        let Received::Part(received) = device.decode(&keyboard_change(mid, low, &data)) else {
            panic!("expected a part change for {id}");
        };
        assert_eq!(received.part, number, "{id}");
        assert_eq!(
            received.values,
            [ParameterChange::new(id, Location::part(number), value)],
            "{id}"
        );
    }
}
