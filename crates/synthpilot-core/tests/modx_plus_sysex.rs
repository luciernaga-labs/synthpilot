use synthpilot_core::devices::modx_plus::{self, Awm2, FmX, ModxPlus, awm2, fm_x};
use synthpilot_core::domain::{
    DeviceProfile, EncodeError, EngineProfile, Location, ParameterChange, ParameterId, Scope,
    Support, ValidationError,
};
use synthpilot_core::protocol::ProtocolError;
use synthpilot_core::protocol::yamaha::DeviceNumber;

fn awm2() -> Awm2 {
    Awm2::new(DeviceNumber::default())
}

fn fm_x() -> FmX {
    FmX::new(DeviceNumber::default())
}

fn encode(engine: &dyn EngineProfile, id: ParameterId, location: Location, value: i32) -> String {
    engine
        .encode(&ParameterChange::new(id, location, value))
        .unwrap_or_else(|error| panic!("{id} = {value} should encode: {error}"))
        .to_hex()
}

fn assert_golden(
    engine: &dyn EngineProfile,
    id: ParameterId,
    location: Location,
    value: i32,
    payload: &str,
) {
    assert_eq!(
        encode(engine, id, location, value),
        format!("F0 43 10 7F 1C 07 {payload} F7"),
        "{id} = {value} at {location:?}"
    );
}

#[test]
fn awm2_golden_messages() {
    let e = awm2();
    let el = Location::element;
    assert_golden(&e, awm2::ELEMENT_LEVEL, el(1, 1), 100, "41 00 28 64");
    assert_golden(&e, awm2::ELEMENT_SWITCH, el(2, 3), 1, "41 21 00 01");
    assert_golden(&e, awm2::XA_CONTROL, el(1, 1), 7, "41 00 0C 07");
    assert_golden(&e, awm2::ELEMENT_PAN, el(1, 1), -63, "41 00 08 01");
    assert_golden(&e, awm2::ELEMENT_PAN, el(1, 1), 0, "41 00 08 40");
    assert_golden(&e, awm2::ELEMENT_PAN, el(1, 1), 63, "41 00 08 7F");
    assert_golden(
        &e,
        awm2::LEVEL_VELOCITY_SENSITIVITY,
        el(1, 1),
        -64,
        "41 00 29 00",
    );
    assert_golden(&e, awm2::AEG_ATTACK_TIME, el(1, 1), 12, "41 00 2D 0C");
    assert_golden(&e, awm2::AEG_DECAY_1_TIME, el(1, 1), 64, "41 00 2E 40");
    assert_golden(&e, awm2::AEG_DECAY_2_TIME, el(1, 1), 64, "41 00 2F 40");
    assert_golden(&e, awm2::AEG_RELEASE_TIME, el(1, 1), 50, "41 00 31 32");
    assert_golden(&e, awm2::COARSE_TUNE, el(16, 8), -12, "41 7F 49 34");
    assert_golden(&e, awm2::COARSE_TUNE, el(1, 1), -48, "41 00 49 10");
    assert_golden(&e, awm2::COARSE_TUNE, el(1, 1), 48, "41 00 49 70");
    assert_golden(&e, awm2::FINE_TUNE, el(1, 1), 63, "41 00 4A 7F");
    assert_golden(&e, awm2::FILTER_CUTOFF, el(3, 2), 200, "42 12 01 01 48");
    assert_golden(&e, awm2::FILTER_CUTOFF, el(1, 1), 127, "42 00 01 00 7F");
    assert_golden(&e, awm2::FILTER_RESONANCE, el(1, 1), 90, "42 00 05 5A");
}

#[test]
fn fm_x_golden_messages() {
    let e = fm_x();
    let part = Location::part;
    let op = Location::element;
    assert_golden(&e, fm_x::ALGORITHM, part(1), 1, "48 00 4F 00");
    assert_golden(&e, fm_x::ALGORITHM, part(2), 88, "48 01 4F 57");
    assert_golden(&e, fm_x::FEEDBACK_LEVEL, part(1), 7, "48 00 50 07");
    assert_golden(&e, fm_x::FILTER_CUTOFF, part(4), 255, "48 03 0C 01 7F");
    assert_golden(&e, fm_x::FILTER_RESONANCE, part(1), 10, "48 00 0F 0A");
    assert_golden(&e, fm_x::FREQUENCY_MODE, op(1, 1), 1, "49 00 03 01");
    assert_golden(&e, fm_x::TUNE_COARSE, op(1, 3), 31, "49 20 04 1F");
    assert_golden(&e, fm_x::TUNE_FINE, op(1, 1), 127, "49 00 05 7F");
    assert_golden(&e, fm_x::DETUNE, op(1, 2), -15, "49 10 06 00");
    assert_golden(&e, fm_x::DETUNE, op(1, 2), 15, "49 10 06 1E");
    assert_golden(&e, fm_x::OPERATOR_LEVEL, op(5, 8), 99, "49 74 1A 63");
    assert_golden(
        &e,
        fm_x::LEVEL_VELOCITY_SENSITIVITY,
        op(1, 1),
        7,
        "49 00 20 0E",
    );
    assert_golden(&e, fm_x::AEG_ATTACK_TIME, op(1, 1), 99, "49 00 14 63");
    assert_golden(&e, fm_x::AEG_DECAY_1_TIME, op(1, 1), 30, "49 00 15 1E");
    assert_golden(&e, fm_x::AEG_RELEASE_TIME, op(16, 1), 40, "49 0F 17 28");
}

#[test]
fn device_number_sets_the_status_nibble() {
    let engine = Awm2::new(DeviceNumber::new(5).unwrap());
    let hex = encode(&engine, awm2::ELEMENT_LEVEL, Location::element(1, 1), 0);
    assert!(hex.starts_with("F0 43 15 "), "{hex}");
}

#[test]
fn defaults_encode_to_the_data_list_default_bytes() {
    let awm2 = awm2();
    let fm_x = fm_x();
    let cases: [(&dyn EngineProfile, ParameterId, &str); 24] = [
        (&awm2, awm2::XA_CONTROL, "00"),
        (&awm2, awm2::ELEMENT_LEVEL, "7F"),
        (&awm2, awm2::ELEMENT_PAN, "40"),
        (&awm2, awm2::LEVEL_VELOCITY_SENSITIVITY, "40"),
        (&awm2, awm2::AEG_ATTACK_TIME, "00"),
        (&awm2, awm2::AEG_DECAY_1_TIME, "40"),
        (&awm2, awm2::AEG_DECAY_2_TIME, "40"),
        (&awm2, awm2::AEG_RELEASE_TIME, "32"),
        (&awm2, awm2::COARSE_TUNE, "40"),
        (&awm2, awm2::FINE_TUNE, "40"),
        (&awm2, awm2::FILTER_CUTOFF, "01 20"),
        (&awm2, awm2::FILTER_RESONANCE, "00"),
        (&fm_x, fm_x::ALGORITHM, "00"),
        (&fm_x, fm_x::FEEDBACK_LEVEL, "00"),
        (&fm_x, fm_x::FILTER_CUTOFF, "01 7F"),
        (&fm_x, fm_x::FILTER_RESONANCE, "0A"),
        (&fm_x, fm_x::FREQUENCY_MODE, "00"),
        (&fm_x, fm_x::TUNE_COARSE, "01"),
        (&fm_x, fm_x::TUNE_FINE, "00"),
        (&fm_x, fm_x::DETUNE, "0F"),
        (&fm_x, fm_x::LEVEL_VELOCITY_SENSITIVITY, "07"),
        (&fm_x, fm_x::AEG_ATTACK_TIME, "00"),
        (&fm_x, fm_x::AEG_DECAY_1_TIME, "00"),
        (&fm_x, fm_x::AEG_RELEASE_TIME, "28"),
    ];
    for (engine, id, data) in cases {
        let definition = engine.parameter(id).unwrap();
        let location = Location::for_scope(definition.scope, 1, 2);
        let value = engine.default_value(definition, location).unwrap();
        let hex = encode(engine, id, location, value);
        assert!(
            hex.ends_with(&format!(" {data} F7")),
            "default of {id}: {hex}"
        );
    }
}

#[test]
fn per_location_defaults_follow_the_data_list_notes() {
    let awm2 = awm2();
    let switch = awm2.parameter(awm2::ELEMENT_SWITCH).unwrap();
    assert_eq!(awm2.default_value(switch, Location::element(1, 1)), Some(1));
    assert_eq!(awm2.default_value(switch, Location::element(1, 2)), Some(0));

    let fm_x = fm_x();
    let level = fm_x.parameter(fm_x::OPERATOR_LEVEL).unwrap();
    assert_eq!(fm_x.default_value(level, Location::element(1, 1)), Some(99));
    assert_eq!(fm_x.default_value(level, Location::element(1, 2)), Some(0));
}

#[test]
fn out_of_range_values_are_rejected() {
    let engine = awm2();
    let cases = [
        (awm2::ELEMENT_LEVEL, 128),
        (awm2::ELEMENT_LEVEL, -1),
        (awm2::ELEMENT_PAN, -64),
        (awm2::COARSE_TUNE, 49),
        (awm2::FILTER_CUTOFF, 256),
    ];
    for (id, value) in cases {
        let change = ParameterChange::new(id, Location::element(1, 1), value);
        assert!(
            matches!(
                engine.encode(&change),
                Err(EncodeError::Invalid(ValidationError::OutOfRange { .. }))
            ),
            "{id} = {value} should be rejected"
        );
    }
}

#[test]
fn invalid_locations_are_rejected() {
    let awm2 = awm2();
    let fm_x = fm_x();
    let level = awm2::ELEMENT_LEVEL;
    let cases: [(&dyn EngineProfile, ParameterId, Location, ValidationError); 5] = [
        (
            &awm2,
            level,
            Location::element(0, 1),
            ValidationError::PartOutOfRange { part: 0, max: 16 },
        ),
        (
            &awm2,
            level,
            Location::element(17, 1),
            ValidationError::PartOutOfRange { part: 17, max: 16 },
        ),
        (
            &awm2,
            level,
            Location::element(1, 9),
            ValidationError::ElementOutOfRange { element: 9, max: 8 },
        ),
        (
            &awm2,
            level,
            Location::part(1),
            ValidationError::MissingElement(level),
        ),
        (
            &fm_x,
            fm_x::ALGORITHM,
            Location::element(1, 1),
            ValidationError::UnexpectedElement(fm_x::ALGORITHM),
        ),
    ];
    for (engine, id, location, error) in cases {
        let value = engine.parameter(id).and_then(|p| p.default).unwrap_or(0);
        let change = ParameterChange::new(id, location, value);
        assert_eq!(engine.encode(&change), Err(EncodeError::Invalid(error)));
    }
}

#[test]
fn unknown_parameters_are_rejected() {
    let change = ParameterChange::new(fm_x::ALGORITHM, Location::part(1), 1);
    assert_eq!(
        awm2().encode(&change),
        Err(EncodeError::Invalid(ValidationError::UnknownParameter(
            fm_x::ALGORITHM
        )))
    );
}

#[test]
fn unimplemented_parameters_never_produce_messages() {
    let awm2 = awm2();
    let fm_x = fm_x();
    let engines: [&dyn EngineProfile; 2] = [&awm2, &fm_x];
    let mut checked = 0;
    for engine in engines {
        for definition in engine.parameters().iter().filter(|p| !p.is_implemented()) {
            let location = Location::for_scope(definition.scope, 1, 1);
            for value in [definition.range.min(), definition.range.max()] {
                let change = ParameterChange::new(definition.id, location, value);
                assert_eq!(
                    engine.encode(&change),
                    Err(EncodeError::NotImplemented(definition.id))
                );
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 4);
}

#[test]
fn implemented_parameters_cite_the_data_list() {
    let device = ModxPlus::new();
    for engine in device.engines() {
        for definition in engine.parameters() {
            match definition.support {
                Support::Implemented(source) => {
                    assert_eq!(source.document, modx_plus::DATA_LIST);
                    assert!(
                        [207, 208, 217, 218, 219, 220].contains(&source.page),
                        "{}",
                        definition.id
                    );
                }
                Support::NotImplemented { reason } => assert!(!reason.is_empty()),
            }
        }
    }
}

#[test]
fn every_implemented_value_in_range_encodes() {
    let awm2 = awm2();
    let fm_x = fm_x();
    let engines: [&dyn EngineProfile; 2] = [&awm2, &fm_x];
    for engine in engines {
        for definition in engine.parameters().iter().filter(|p| p.is_implemented()) {
            let location = match definition.scope {
                Scope::Part => Location::part(16),
                Scope::Element => Location::element(16, 8),
            };
            for value in definition.range.min()..=definition.range.max() {
                let change = ParameterChange::new(definition.id, location, value);
                let result = engine.encode(&change);
                assert!(result.is_ok(), "{} = {value}: {result:?}", definition.id);
            }
        }
    }
}

#[test]
fn invalid_device_numbers_are_rejected() {
    assert_eq!(
        DeviceNumber::new(16),
        Err(ProtocolError::DeviceNumberOutOfRange(16))
    );
}
