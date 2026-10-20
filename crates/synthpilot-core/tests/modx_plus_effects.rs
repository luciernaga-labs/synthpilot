use synthpilot_core::devices::modx_plus::{
    self, Awm2, Effects, FmX, ModxPlus, awm2, effects, part,
};
use synthpilot_core::domain::{
    DeviceProfile, EditError, EncodeError, EngineProfile, Location, ParameterChange, ParameterId,
    Received, Sound, SoundReader, ValidationError,
};
use synthpilot_core::protocol::yamaha::{self, Address, DeviceNumber, ModelId};

const MODEL: ModelId = ModelId::new(&[0x7F, 0x1C, 0x07]);

fn encode(engine: &dyn EngineProfile, id: ParameterId, location: Location, value: i32) -> String {
    engine
        .encode(&ParameterChange::new(id, location, value))
        .unwrap()
        .to_hex()
}

fn dump(high: u8, mid: u8, low: u8, data: &[u8]) -> Vec<u8> {
    yamaha::bulk_dump(
        DeviceNumber::default(),
        MODEL,
        Address::new(high, mid, low).unwrap(),
        data,
    )
    .unwrap()
    .into_bytes()
}

fn change(high: u8, mid: u8, low: u8, data: &[u8]) -> Vec<u8> {
    yamaha::parameter_change(
        DeviceNumber::default(),
        MODEL,
        Address::new(high, mid, low).unwrap(),
        data,
    )
    .unwrap()
    .into_bytes()
}

#[test]
fn part_effect_settings_match_the_performance_part_table() {
    let awm2 = Awm2::new(DeviceNumber::default());
    let part = Location::part(1);
    let cases = [
        (part::INSERTION_SWITCH, 3, "31 00 26 03"),
        (part::REVERB_SEND, 40, "31 00 29 28"),
        (part::VARIATION_SEND, 64, "31 00 2A 40"),
        (part::DRY_LEVEL, 127, "31 00 2C 7F"),
        (part::INSERTION_CONNECTION, 1, "31 00 3B 01"),
        (part::EQ_LOW_GAIN, 32, "31 00 55 60"),
        (part::EQ_MID_GAIN, 0, "31 00 59 40"),
        (part::EQ_HIGH_GAIN, -32, "31 00 5D 20"),
    ];
    for (id, value, bytes) in cases {
        assert_eq!(
            encode(&awm2, id, part, value),
            format!("F0 43 10 7F 1C 07 {bytes} F7"),
            "{id}"
        );
    }
}

#[test]
fn insertion_types_use_the_insertion_blocks_of_each_part() {
    let awm2 = Awm2::new(DeviceNumber::default());
    assert_eq!(
        encode(&awm2, part::INSERTION_A_TYPE, Location::part(2), 15),
        "F0 43 10 7F 1C 07 31 21 00 03 00 F7"
    );
    assert_eq!(
        encode(&awm2, part::INSERTION_B_TYPE, Location::part(16), 1),
        "F0 43 10 7F 1C 07 31 3F 00 01 02 F7"
    );
    assert_eq!(
        encode(&awm2, part::INSERTION_A_TYPE, Location::part(1), 0),
        "F0 43 10 7F 1C 07 31 20 00 00 00 F7"
    );
}

#[test]
fn elements_choose_their_insertion() {
    let awm2 = Awm2::new(DeviceNumber::default());
    assert_eq!(
        encode(&awm2, awm2::ELEMENT_CONNECTION, Location::element(1, 2), 2),
        "F0 43 10 7F 1C 07 41 10 17 02 F7"
    );
    let definition = awm2.parameter(awm2::ELEMENT_CONNECTION).unwrap();
    assert_eq!(definition.default, Some(1));
    assert_eq!(definition.display_value(0), "Thru");
}

#[test]
fn fm_parts_cannot_run_insertions_in_parallel() {
    let fm_x = FmX::new(DeviceNumber::default());
    let change = ParameterChange::new(part::INSERTION_CONNECTION, Location::part(1), 0);
    assert_eq!(
        fm_x.encode(&change),
        Err(EncodeError::Invalid(ValidationError::OutOfRange {
            parameter: part::INSERTION_CONNECTION,
            value: 0,
            min: 1,
            max: 2,
        }))
    );
    assert_eq!(
        encode(&fm_x, part::INSERTION_CONNECTION, Location::part(1), 2),
        "F0 43 10 7F 1C 07 31 00 3B 02 F7"
    );
}

#[test]
fn effect_defaults_encode_to_the_data_list_defaults() {
    let effects = Effects::new(DeviceNumber::default());
    let awm2 = Awm2::new(DeviceNumber::default());
    let cases: [(&dyn EngineProfile, ParameterId, Location, &str); 9] = [
        (
            &effects,
            effects::REVERB_TYPE,
            Location::performance(),
            "30 41 00 01 00",
        ),
        (
            &effects,
            effects::REVERB_RETURN,
            Location::performance(),
            "30 41 44 40",
        ),
        (
            &effects,
            effects::VARIATION_TYPE,
            Location::performance(),
            "30 42 00 03 00",
        ),
        (
            &effects,
            effects::VARIATION_RETURN,
            Location::performance(),
            "30 42 44 60",
        ),
        (
            &awm2,
            part::INSERTION_SWITCH,
            Location::part(1),
            "31 00 26 03",
        ),
        (&awm2, part::REVERB_SEND, Location::part(1), "31 00 29 0C"),
        (&awm2, part::DRY_LEVEL, Location::part(1), "31 00 2C 7F"),
        (&awm2, part::EQ_LOW_GAIN, Location::part(1), "31 00 55 40"),
        (
            &awm2,
            part::INSERTION_A_TYPE,
            Location::part(1),
            "31 20 00 00 00",
        ),
    ];
    for (engine, id, location, bytes) in cases {
        let default = engine.parameter(id).and_then(|p| p.default).unwrap();
        assert_eq!(
            encode(engine, id, location, default),
            format!("F0 43 10 7F 1C 07 {bytes} F7"),
            "{id}"
        );
    }
}

#[test]
fn effect_values_read_naturally() {
    let effects = Effects::new(DeviceNumber::default());
    let reverb = effects.parameter(effects::REVERB_TYPE).unwrap();
    assert_eq!(reverb.display_value(2), "REV-X HALL");
    assert_eq!(reverb.display_range(), "NO EFFECT … SPACE SIMULATOR");
    let awm2 = Awm2::new(DeviceNumber::default());
    let gain = awm2.parameter(part::EQ_LOW_GAIN).unwrap();
    assert_eq!(gain.display_value(32), "+12.00 dB");
    assert_eq!(gain.display_value(-1), "-0.38 dB");
    let insertion = awm2.parameter(part::INSERTION_A_TYPE).unwrap();
    assert_eq!(insertion.display_value(0), "THRU");
    assert_eq!(insertion.display_value(32), "ROTARY SPEAKER 1");
}

#[test]
fn reads_reverb_and_variation_with_the_sound() {
    let device = ModxPlus::new();
    let mut reverb = vec![0x00; 0x47];
    reverb[..2].copy_from_slice(&[0x01, 0x21]);
    reverb[0x44] = 0x50;
    reverb[0x46] = 0x00;
    let mut variation = vec![0x00; 0x48];
    variation[..2].copy_from_slice(&[0x02, 0x28]);
    variation[0x45] = 0x40;
    variation[0x47] = 0x01;
    let mut insertion = vec![0x00; 0x44];
    insertion[..2].copy_from_slice(&[0x06, 0x20]);

    let mut reader = SoundReader::new();
    for message in [
        dump(0x0E, 0x52, 0x00, &[]),
        dump(0x31, 0x00, 0x00, &[0x40; 0x79]),
        dump(0x31, 0x20, 0x00, &insertion),
        dump(0x41, 0x00, 0x00, &[0x01; 0x67]),
        dump(0x0F, 0x52, 0x00, &[]),
        dump(0x30, 0x41, 0x00, &reverb),
        dump(0x30, 0x42, 0x00, &variation),
    ] {
        assert!(reader.accept(device.decode(&message)));
    }
    let sound = reader.into_sound(&device);
    let common = sound.common().expect("the MODX+ has performance effects");
    assert_eq!(common.engine(), effects::ENGINE_ID);
    assert_eq!(common.value(effects::REVERB_TYPE, None), Some(9));
    assert_eq!(common.value(effects::REVERB_RETURN, None), Some(0x50));
    assert_eq!(common.value(effects::REVERB_SWITCH, None), Some(0));
    assert_eq!(common.value(effects::VARIATION_TYPE, None), Some(9));
    assert_eq!(common.value(effects::VARIATION_PAN, None), Some(0));
    assert_eq!(common.value(effects::VARIATION_SWITCH, None), Some(1));
    let part = sound.part(1).unwrap();
    assert_eq!(part.value(part::INSERTION_A_TYPE, None), Some(32));
    assert_eq!(part.value(part::REVERB_SEND, None), Some(0x40));
}

#[test]
fn keyboard_edits_to_effects_are_decoded() {
    let device = ModxPlus::new();
    assert_eq!(
        device.decode(&change(0x30, 0x41, 0x44, &[0x10])),
        Received::Common(vec![ParameterChange::new(
            effects::REVERB_RETURN,
            Location::performance(),
            0x10
        )])
    );
    let Received::Part(data) = device.decode(&change(0x31, 0x32, 0x00, &[0x03, 0x00])) else {
        panic!("expected a part change");
    };
    assert_eq!(data.part, 3);
    assert_eq!(
        data.values,
        [ParameterChange::new(
            part::INSERTION_B_TYPE,
            Location::part(3),
            15
        )]
    );
    assert_eq!(
        device.decode(&change(0x30, 0x41, 0x00, &[0x7F, 0x7F])),
        Received::Ignored
    );
}

#[test]
fn performance_effects_are_edited_on_the_sound() {
    let device = ModxPlus::new();
    let effects = device.common().unwrap();
    let mut sound = Sound::new(&device);
    let common = sound.common().unwrap();
    assert_eq!(common.value(effects::VARIATION_TYPE, None), Some(15));

    let message = sound
        .apply(
            effects,
            ParameterChange::new(effects::REVERB_TYPE, Location::performance(), 12),
        )
        .unwrap();
    assert_eq!(message.to_hex(), "F0 43 10 7F 1C 07 30 41 00 01 40 F7");
    assert_eq!(
        sound.common().unwrap().value(effects::REVERB_TYPE, None),
        Some(12)
    );

    sound
        .store(
            effects,
            ParameterChange::new(effects::VARIATION_RETURN, Location::performance(), 5),
        )
        .unwrap();
    assert_eq!(
        sound
            .common()
            .unwrap()
            .value(effects::VARIATION_RETURN, None),
        Some(5)
    );

    let awm2 = device.engine(awm2::ENGINE_ID).unwrap();
    assert_eq!(
        sound.apply(
            awm2,
            ParameterChange::new(effects::REVERB_TYPE, Location::performance(), 1),
        ),
        Err(EditError::EngineMismatch {
            part: 0,
            expected: awm2::ENGINE_ID,
            actual: effects::ENGINE_ID,
        })
    );
    assert_eq!(
        sound.load_common(
            effects,
            &[
                ParameterChange::new(effects::REVERB_RETURN, Location::performance(), 7),
                ParameterChange::new(effects::REVERB_RETURN, Location::part(1), 7),
            ]
        ),
        1
    );
    assert_eq!(modx_plus::PART_COUNT, sound.part_count());
}
