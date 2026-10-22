use synthpilot_core::devices::modx_plus::{Awm2, FmX, awm2, fm_x, part, waves};
use synthpilot_core::domain::{EngineProfile, Location, ParameterChange, ParameterId};
use synthpilot_core::protocol::yamaha::DeviceNumber;

fn encode(engine: &dyn EngineProfile, id: ParameterId, location: Location, value: i32) -> String {
    engine
        .encode(&ParameterChange::new(id, location, value))
        .unwrap()
        .to_hex()
}

fn assert_bytes(engine: &dyn EngineProfile, cases: &[(ParameterId, Location, i32, &str)]) {
    for (id, location, value, bytes) in cases {
        assert_eq!(
            encode(engine, *id, *location, *value),
            format!("F0 43 10 7F 1C 07 {bytes} F7"),
            "{id}"
        );
    }
}

fn assert_defaults(engine: &dyn EngineProfile, location: Location, cases: &[(ParameterId, &str)]) {
    for (id, bytes) in cases {
        let default = engine.parameter(*id).and_then(|p| p.default).unwrap();
        assert_eq!(
            encode(engine, *id, location, default),
            format!("F0 43 10 7F 1C 07 {bytes} F7"),
            "{id}"
        );
    }
}

#[test]
fn awm2_waves_are_chosen_by_number_and_name() {
    let awm2 = Awm2::new(DeviceNumber::default());
    let element = Location::element(1, 1);
    assert_bytes(
        &awm2,
        &[
            (awm2::WAVE_NUMBER, element, 1, "41 00 03 00 01"),
            (awm2::WAVE_NUMBER, element, 166, "41 00 03 01 26"),
            (
                awm2::WAVE_NUMBER,
                Location::element(2, 3),
                6347,
                "41 21 03 31 4B",
            ),
            (awm2::WAVE_SELECT, element, 0, "41 00 01 00"),
            (awm2::WAVE_SELECT, element, 9, "41 00 01 09"),
        ],
    );
    let wave = awm2.parameter(awm2::WAVE_NUMBER).unwrap();
    assert_eq!(wave.display_value(166), "166 EP1 Sw");
    assert_eq!(wave.display_value(264), "264 DX1");
    assert_eq!(wave.range.max(), 6347);
    assert_eq!(waves::all().len(), waves::COUNT);
    assert!(wave.is_implemented());
}

#[test]
fn awm2_envelopes_filter_and_lfo_match_the_element_tables() {
    let awm2 = Awm2::new(DeviceNumber::default());
    let element = Location::element(1, 2);
    assert_bytes(
        &awm2,
        &[
            (awm2::AEG_DECAY_2_LEVEL, element, 0, "41 10 35 00"),
            (
                awm2::PITCH_VELOCITY_SENSITIVITY,
                element,
                -64,
                "41 10 4B 00",
            ),
            (awm2::FILTER_CUTOFF_VELOCITY, element, 20, "42 10 03 54"),
            (awm2::FEG_HOLD_LEVEL, element, -128, "42 10 13 00 00"),
            (awm2::FEG_ATTACK_LEVEL, element, 127, "42 10 15 01 7F"),
            (awm2::FEG_DECAY_2_LEVEL, element, -1, "42 10 19 00 7F"),
            (awm2::FEG_DEPTH, element, 63, "42 10 1D 7F"),
            (awm2::LFO_WAVE, element, 2, "42 10 3A 02"),
            (awm2::LFO_SPEED, element, 63, "42 10 3D 3F"),
            (awm2::LFO_AMPLITUDE, element, 30, "42 10 3E 1E"),
            (awm2::LFO_FADE_IN_TIME, element, 5, "42 10 41 05"),
        ],
    );
    assert_defaults(
        &awm2,
        Location::element(1, 1),
        &[
            (awm2::WAVE_NUMBER, "41 00 03 00 01"),
            (awm2::AEG_INITIAL_LEVEL, "41 00 32 00"),
            (awm2::AEG_ATTACK_LEVEL, "41 00 33 7F"),
            (awm2::AEG_DECAY_1_LEVEL, "41 00 34 7F"),
            (awm2::AEG_DECAY_2_LEVEL, "41 00 35 7F"),
            (awm2::PITCH_VELOCITY_SENSITIVITY, "41 00 4B 40"),
            (awm2::FILTER_CUTOFF_VELOCITY, "42 00 03 40"),
            (awm2::FEG_HOLD_TIME, "42 00 0E 00"),
            (awm2::FEG_ATTACK_TIME, "42 00 0F 00"),
            (awm2::FEG_DECAY_1_TIME, "42 00 10 40"),
            (awm2::FEG_DECAY_2_TIME, "42 00 11 40"),
            (awm2::FEG_RELEASE_TIME, "42 00 12 50"),
            (awm2::FEG_HOLD_LEVEL, "42 00 13 01 00"),
            (awm2::FEG_ATTACK_LEVEL, "42 00 15 01 7F"),
            (awm2::FEG_DECAY_1_LEVEL, "42 00 17 01 7F"),
            (awm2::FEG_DECAY_2_LEVEL, "42 00 19 01 7F"),
            (awm2::FEG_RELEASE_LEVEL, "42 00 1B 01 00"),
            (awm2::FEG_DEPTH, "42 00 1D 68"),
            (awm2::FEG_DEPTH_VELOCITY, "42 00 20 40"),
            (awm2::LFO_WAVE, "42 00 3A 01"),
            (awm2::LFO_DELAY_TIME, "42 00 3C 00"),
            (awm2::LFO_SPEED, "42 00 3D 26"),
            (awm2::LFO_AMPLITUDE, "42 00 3E 00"),
            (awm2::LFO_PITCH, "42 00 3F 00"),
            (awm2::LFO_FILTER, "42 00 40 00"),
            (awm2::LFO_FADE_IN_TIME, "42 00 41 00"),
        ],
    );
}

#[test]
fn fm_x_common_filter_eg_and_lfo_match_the_fm_part_common_table() {
    let fm_x = FmX::new(DeviceNumber::default());
    let part = Location::part(2);
    assert_bytes(
        &fm_x,
        &[
            (fm_x::FILTER_CUTOFF_VELOCITY, part, 10, "48 01 0E 4A"),
            (fm_x::FEG_ATTACK_LEVEL, part, 64, "48 01 1E 01 40"),
            (fm_x::LFO_WAVE, part, 5, "48 01 47 05"),
            (fm_x::LFO_PITCH, part, 99, "48 01 4C 63"),
        ],
    );
    assert_defaults(
        &fm_x,
        Location::part(1),
        &[
            (fm_x::PITCH_VELOCITY_SENSITIVITY, "48 00 06 40"),
            (fm_x::FILTER_CUTOFF_VELOCITY, "48 00 0E 40"),
            (fm_x::FEG_HOLD_TIME, "48 00 17 00"),
            (fm_x::FEG_ATTACK_TIME, "48 00 18 00"),
            (fm_x::FEG_DECAY_1_TIME, "48 00 19 00"),
            (fm_x::FEG_DECAY_2_TIME, "48 00 1A 00"),
            (fm_x::FEG_RELEASE_TIME, "48 00 1B 00"),
            (fm_x::FEG_HOLD_LEVEL, "48 00 1C 01 00"),
            (fm_x::FEG_ATTACK_LEVEL, "48 00 1E 01 00"),
            (fm_x::FEG_DECAY_1_LEVEL, "48 00 20 01 00"),
            (fm_x::FEG_DECAY_2_LEVEL, "48 00 22 01 00"),
            (fm_x::FEG_RELEASE_LEVEL, "48 00 24 01 00"),
            (fm_x::FEG_DEPTH, "48 00 26 68"),
            (fm_x::FEG_DEPTH_VELOCITY, "48 00 29 40"),
            (fm_x::LFO_WAVE, "48 00 47 00"),
            (fm_x::LFO_SPEED, "48 00 48 1E"),
            (fm_x::LFO_DELAY_TIME, "48 00 4A 00"),
            (fm_x::LFO_PITCH, "48 00 4C 00"),
            (fm_x::LFO_AMPLITUDE, "48 00 4D 00"),
            (fm_x::LFO_FILTER, "48 00 4E 00"),
        ],
    );
}

#[test]
fn fm_x_operators_match_the_fm_part_operator_table() {
    let fm_x = FmX::new(DeviceNumber::default());
    assert_bytes(
        &fm_x,
        &[
            (
                fm_x::SPECTRAL_FORM,
                Location::element(1, 3),
                6,
                "49 20 09 06",
            ),
            (
                fm_x::AEG_DECAY_2_LEVEL,
                Location::element(4, 8),
                0,
                "49 73 12 00",
            ),
            (
                fm_x::LEVEL_SCALING_HIGH_DEPTH,
                Location::element(1, 1),
                40,
                "49 00 1D 28",
            ),
        ],
    );
    assert_defaults(
        &fm_x,
        Location::element(1, 1),
        &[
            (fm_x::SPECTRAL_FORM, "49 00 09 00"),
            (fm_x::SPECTRAL_SKIRT, "49 00 0A 00"),
            (fm_x::SPECTRAL_RESONANCE, "49 00 0B 00"),
            (fm_x::AEG_ATTACK_LEVEL, "49 00 10 63"),
            (fm_x::AEG_DECAY_1_LEVEL, "49 00 11 63"),
            (fm_x::AEG_DECAY_2_LEVEL, "49 00 12 63"),
            (fm_x::AEG_RELEASE_LEVEL, "49 00 13 00"),
            (fm_x::AEG_DECAY_2_TIME, "49 00 16 00"),
            (fm_x::AEG_HOLD_TIME, "49 00 18 00"),
            (fm_x::AEG_TIME_KEY_FOLLOW, "49 00 19 00"),
            (fm_x::LEVEL_SCALING_LOW_DEPTH, "49 00 1C 00"),
            (fm_x::LEVEL_SCALING_HIGH_DEPTH, "49 00 1D 00"),
            (fm_x::OPERATOR_LFO_PITCH, "49 00 21 03"),
            (fm_x::OPERATOR_LFO_AMPLITUDE, "49 00 22 03"),
        ],
    );
    let form = fm_x.parameter(fm_x::SPECTRAL_FORM).unwrap();
    assert_eq!(form.display_value(3), "Odd 1");
}

#[test]
fn awm2_key_and_velocity_ranges_match_the_element_table() {
    let awm2 = Awm2::new(DeviceNumber::default());
    let element = Location::element(1, 3);
    assert_bytes(
        &awm2,
        &[
            (awm2::NOTE_LIMIT_LOW, element, 60, "41 20 0D 3C"),
            (awm2::VELOCITY_LIMIT_LOW, element, 90, "41 20 0F 5A"),
            (awm2::VELOCITY_LIMIT_HIGH, element, 1, "41 20 10 01"),
            (awm2::VELOCITY_CROSS_FADE, element, 20, "41 20 11 14"),
            (
                awm2::AEG_TIME_VELOCITY_SENSITIVITY,
                element,
                30,
                "41 20 38 5E",
            ),
        ],
    );
    assert_defaults(
        &awm2,
        Location::element(1, 1),
        &[
            (awm2::NOTE_LIMIT_LOW, "41 00 0D 00"),
            (awm2::NOTE_LIMIT_HIGH, "41 00 0E 7F"),
            (awm2::VELOCITY_LIMIT_LOW, "41 00 0F 01"),
            (awm2::VELOCITY_LIMIT_HIGH, "41 00 10 7F"),
            (awm2::VELOCITY_CROSS_FADE, "41 00 11 00"),
            (awm2::AEG_TIME_VELOCITY_SEGMENT, "41 00 37 04"),
            (awm2::AEG_TIME_VELOCITY_SENSITIVITY, "41 00 38 40"),
            (awm2::AEG_TIME_KEY_FOLLOW, "41 00 39 40"),
        ],
    );
    assert_defaults(
        &awm2,
        Location::part(1),
        &[
            (part::VELOCITY_DEPTH, "31 00 22 40"),
            (part::VELOCITY_OFFSET, "31 00 23 40"),
        ],
    );
    let note = awm2.parameter(awm2::NOTE_LIMIT_HIGH).unwrap();
    assert_eq!(note.display_range(), "C-2 … G8");
    assert_eq!(
        awm2.parameter(awm2::AEG_TIME_VELOCITY_SEGMENT)
            .unwrap()
            .display_value(1),
        "Atk+Dcy"
    );
}
