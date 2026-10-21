use synthpilot_core::devices::modx_plus::{ModxPlus, awm2, effects, part};
use synthpilot_core::domain::{
    DeviceProfile, EditError, EngineProfile, Location, ParameterChange, Sound, ValidationError,
};

fn hex(engine: &dyn EngineProfile, change: ParameterChange) -> String {
    engine.encode(&change).unwrap().to_hex()
}

#[test]
fn effect_parameter_slots_use_the_documented_addresses() {
    let device = ModxPlus::new();
    let effects = device.common().unwrap();
    let awm2 = device.engine(awm2::ENGINE_ID).unwrap();
    let performance = Location::performance();
    assert_eq!(
        hex(
            effects,
            ParameterChange::new(effects::REVERB_PARAMETERS[0], performance, 69)
        ),
        "F0 43 10 7F 1C 07 30 41 03 00 45 F7"
    );
    assert_eq!(
        hex(
            effects,
            ParameterChange::new(effects::REVERB_PARAMETERS[23], performance, 200)
        ),
        "F0 43 10 7F 1C 07 30 41 31 01 48 F7"
    );
    assert_eq!(
        hex(
            effects,
            ParameterChange::new(effects::VARIATION_PARAMETERS[0], performance, 64)
        ),
        "F0 43 10 7F 1C 07 30 42 03 00 40 F7"
    );
    assert_eq!(
        hex(
            awm2,
            ParameterChange::new(part::INSERTION_A_PARAMETERS[0], Location::part(2), 100)
        ),
        "F0 43 10 7F 1C 07 31 21 03 00 64 F7"
    );
    assert_eq!(
        hex(
            awm2,
            ParameterChange::new(part::INSERTION_B_PARAMETERS[9], Location::part(16), 16383)
        ),
        "F0 43 10 7F 1C 07 31 3F 15 7F 7F F7"
    );
}

#[test]
fn slots_take_the_name_and_range_of_the_selected_type() {
    let device = ModxPlus::new();
    let effects = device.common().unwrap();
    let sound = Sound::new(&device);
    let common = sound.common().unwrap();
    let slot = |index: usize| {
        let generic = effects
            .parameter(effects::VARIATION_PARAMETERS[index])
            .unwrap();
        effects.refine(generic, common)
    };
    let speed = slot(0).unwrap();
    assert_eq!(speed.name, "LFO Speed");
    assert_eq!((speed.range.min(), speed.range.max()), (0, 127));
    assert_eq!(speed.help, Some("0.0Hz – 39.70Hz"));
    assert!(slot(1).is_none());
    assert_eq!(slot(5).unwrap().name, "EQ Low Frequency");

    let reverb = effects
        .refine(
            effects.parameter(effects::REVERB_PARAMETERS[0]).unwrap(),
            common,
        )
        .unwrap();
    assert_eq!(reverb.name, "Reverb Time");

    let volume = effects.parameter(effects::REVERB_RETURN).unwrap();
    assert_eq!(effects.refine(volume, common), Some(*volume));
}

#[test]
fn edits_are_checked_against_the_selected_type() {
    let device = ModxPlus::new();
    let effects = device.common().unwrap();
    let mut sound = Sound::new(&device);
    let performance = Location::performance();
    assert_eq!(
        sound.apply(
            effects,
            ParameterChange::new(effects::VARIATION_PARAMETERS[1], performance, 10)
        ),
        Err(EditError::Unavailable(effects::VARIATION_PARAMETERS[1]))
    );
    assert_eq!(
        sound.apply(
            effects,
            ParameterChange::new(effects::VARIATION_PARAMETERS[0], performance, 300)
        ),
        Err(EditError::Invalid(ValidationError::OutOfRange {
            parameter: effects::VARIATION_PARAMETERS[0],
            value: 300,
            min: 0,
            max: 127,
        }))
    );
    sound
        .apply(
            effects,
            ParameterChange::new(effects::VARIATION_TYPE, performance, 17),
        )
        .unwrap();
    let depth = sound
        .apply(
            effects,
            ParameterChange::new(effects::VARIATION_PARAMETERS[1], performance, 90),
        )
        .unwrap();
    assert_eq!(depth.to_hex(), "F0 43 10 7F 1C 07 30 42 05 00 5A F7");
    assert!(effects.reshapes(effects::VARIATION_TYPE));
}

#[test]
fn types_list_the_parameters_they_bring() {
    let device = ModxPlus::new();
    let effects = device.common().unwrap();
    let variation = effects.parameter(effects::VARIATION_TYPE).unwrap();
    let chorus: Vec<_> = effects
        .dependents(variation, 17)
        .iter()
        .map(|parameter| parameter.name)
        .collect();
    assert_eq!(
        &chorus[..4],
        ["LFO Speed", "LFO Depth", "Feedback Level", "Delay Offset"]
    );
    assert_eq!(
        effects.dependents(variation, 0),
        Vec::<synthpilot_core::domain::ParameterDefinition>::new()
    );
    assert_eq!(
        effects.dependents(variation, 999),
        Vec::<synthpilot_core::domain::ParameterDefinition>::new()
    );
    let volume = effects.parameter(effects::REVERB_RETURN).unwrap();
    assert_eq!(
        effects.dependents(volume, 1),
        Vec::<synthpilot_core::domain::ParameterDefinition>::new()
    );

    let awm2 = device.engine(awm2::ENGINE_ID).unwrap();
    let insertion = awm2.parameter(part::INSERTION_A_TYPE).unwrap();
    let rotary: Vec<_> = awm2
        .dependents(insertion, 32)
        .iter()
        .map(|parameter| parameter.name)
        .collect();
    assert_eq!(rotary[0], "Rotor Speed Slow");
    assert!(awm2.reshapes(part::INSERTION_A_TYPE));
    assert!(!awm2.reshapes(part::VOLUME));

    let mut sound = Sound::new(&device);
    sound.load(1, awm2, None, &[]).unwrap();
    let thru = awm2.refine(
        awm2.parameter(part::INSERTION_A_PARAMETERS[0]).unwrap(),
        sound.part(1).unwrap(),
    );
    assert!(thru.is_none());
}
