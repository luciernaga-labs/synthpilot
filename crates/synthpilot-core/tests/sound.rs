use synthpilot_core::devices::modx_plus::{ModxPlus, awm2, fm_x};
use synthpilot_core::domain::{
    DeviceProfile, EditError, Location, ParameterChange, Sound, ValidationError,
};

#[test]
fn new_awm2_sound_starts_from_data_list_defaults() {
    let device = ModxPlus::new();
    let engine = device.engine(awm2::ENGINE_ID).unwrap();
    let sound = Sound::with_engine(&device, engine);

    assert_eq!(sound.device(), device.id());
    assert_eq!(sound.part_count(), 16);
    let part = sound.part(1).expect("part 1 is initialized");
    assert_eq!(part.engine(), awm2::ENGINE_ID);
    assert_eq!(part.value(awm2::ELEMENT_SWITCH, Some(1)), Some(1));
    assert_eq!(part.value(awm2::ELEMENT_SWITCH, Some(2)), Some(0));
    assert_eq!(part.value(awm2::ELEMENT_LEVEL, Some(8)), Some(127));
    assert_eq!(part.value(awm2::FILTER_CUTOFF, Some(1)), Some(160));
    assert_eq!(part.value(awm2::WAVE_NUMBER, Some(1)), Some(1));
    assert_eq!(part.value(awm2::FILTER_TYPE, Some(1)), None);
    assert!((2..=16).all(|number| sound.part(number).is_none()));
}

#[test]
fn new_fm_x_sound_has_common_and_operator_values() {
    let device = ModxPlus::new();
    let engine = device.engine(fm_x::ENGINE_ID).unwrap();
    let sound = Sound::with_engine(&device, engine);

    let part = sound.part(1).unwrap();
    assert_eq!(part.value(fm_x::ALGORITHM, None), Some(1));
    assert_eq!(part.value(fm_x::OPERATOR_LEVEL, Some(1)), Some(99));
    assert_eq!(part.value(fm_x::OPERATOR_LEVEL, Some(2)), Some(0));
}

#[test]
fn applying_a_change_stores_it_and_returns_the_preview() {
    let device = ModxPlus::new();
    let engine = device.engine(awm2::ENGINE_ID).unwrap();
    let mut sound = Sound::with_engine(&device, engine);

    let change = ParameterChange::new(awm2::ELEMENT_LEVEL, Location::element(1, 1), 100);
    let message = sound.apply(engine, change).unwrap();

    assert_eq!(message.to_hex(), "F0 43 10 7F 1C 07 41 00 28 64 F7");
    assert_eq!(
        sound.part(1).unwrap().value(awm2::ELEMENT_LEVEL, Some(1)),
        Some(100)
    );
}

#[test]
fn rejected_changes_leave_the_sound_untouched() {
    let device = ModxPlus::new();
    let engine = device.engine(awm2::ENGINE_ID).unwrap();
    let mut sound = Sound::with_engine(&device, engine);
    let before = sound.clone();

    let change = ParameterChange::new(awm2::ELEMENT_LEVEL, Location::element(1, 1), 500);
    assert!(sound.apply(engine, change).is_err());
    assert_eq!(sound, before);
}

#[test]
fn parts_can_mix_engines() {
    let device = ModxPlus::new();
    let awm2_engine = device.engine(awm2::ENGINE_ID).unwrap();
    let fm_x_engine = device.engine(fm_x::ENGINE_ID).unwrap();
    let mut sound = Sound::with_engine(&device, awm2_engine);
    sound.assign(2, fm_x_engine).unwrap();

    let change = ParameterChange::new(fm_x::ALGORITHM, Location::part(2), 5);
    assert!(sound.apply(fm_x_engine, change).is_ok());

    let wrong = ParameterChange::new(fm_x::ALGORITHM, Location::part(1), 5);
    assert_eq!(
        sound.apply(fm_x_engine, wrong),
        Err(EditError::EngineMismatch {
            part: 1,
            expected: fm_x::ENGINE_ID,
            actual: awm2::ENGINE_ID,
        })
    );
}

#[test]
fn empty_and_missing_parts_are_reported() {
    let device = ModxPlus::new();
    let engine = device.engine(awm2::ENGINE_ID).unwrap();
    let mut sound = Sound::new(&device);

    let change = ParameterChange::new(awm2::ELEMENT_LEVEL, Location::element(3, 1), 1);
    assert_eq!(sound.apply(engine, change), Err(EditError::EmptyPart(3)));
    assert_eq!(
        sound.assign(17, engine),
        Err(ValidationError::PartOutOfRange { part: 17, max: 16 })
    );

    sound.assign(3, engine).unwrap();
    sound.clear(3).unwrap();
    assert!(sound.part(3).is_none());
}
