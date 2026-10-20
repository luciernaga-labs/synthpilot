use synthpilot_core::devices::modx_plus::{self, ModxPlus, awm2, drum, fm_x, part};
use synthpilot_core::domain::{
    DeviceProfile, Location, ParameterChange, PartData, Received, SoundReader,
};
use synthpilot_core::protocol::SysExMessage;
use synthpilot_core::protocol::yamaha::{self, Address, DeviceNumber, ModelId};

const MODEL: ModelId = ModelId::new(&[0x7F, 0x1C, 0x07]);

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

fn block(size: usize, values: &[(usize, u8)]) -> Vec<u8> {
    let mut data = vec![0x40; size];
    for &(offset, value) in values {
        data[offset] = value;
    }
    data
}

fn part_block(name: &str, values: &[(usize, u8)]) -> Vec<u8> {
    let mut data = block(0x79, values);
    data[..20].fill(b' ');
    data[..name.len()].copy_from_slice(name.as_bytes());
    data
}

fn read(device: &ModxPlus, messages: &[Vec<u8>]) -> SoundReader {
    let mut reader = SoundReader::new();
    for message in messages {
        reader.accept(device.decode(message));
    }
    reader
}

#[test]
fn asks_for_every_part_of_the_edit_buffer() {
    let requests: Vec<_> = ModxPlus::new()
        .read_requests()
        .iter()
        .map(SysExMessage::to_hex)
        .collect();
    assert_eq!(requests.len(), 17);
    assert_eq!(requests[0], "F0 43 20 7F 1C 07 0E 52 00 F7");
    assert_eq!(requests[15], "F0 43 20 7F 1C 07 0E 52 0F F7");
    assert_eq!(requests[16], "F0 43 20 7F 1C 07 0E 53 00 F7");
}

#[test]
fn reads_the_performance_name() {
    let device = ModxPlus::new();
    let mut common = vec![0x00; 0x5E];
    common[..12].copy_from_slice(b"Bright Piano");
    let reader = read(
        &device,
        &[
            dump(0x0E, 0x53, 0x00, &[]),
            dump(0x30, 0x40, 0x00, &common),
            dump(0x30, 0x41, 0x00, &[0x00; 0x47]),
            dump(0x0F, 0x53, 0x00, &[]),
        ],
    );
    assert_eq!(reader.finished(), 1);
    assert_eq!(reader.into_sound(&device).name(), Some("Bright Piano"));
    assert_eq!(
        device.decode(&dump(0x30, 0x40, 0x00, &[0x00; 0x5E])),
        Received::Ignored
    );
}

#[test]
fn reads_an_awm2_part_with_its_name_and_values() {
    let device = ModxPlus::new();
    let reader = read(
        &device,
        &[
            dump(0x0E, 0x52, 0x00, &[]),
            dump(
                0x31,
                0x00,
                0x00,
                &part_block("CFX Concert", &[(0x24, 90), (0x50, 0x30)]),
            ),
            dump(0x41, 0x00, 0x00, &block(0x67, &[(0x00, 1), (0x28, 0x50)])),
            dump(
                0x42,
                0x00,
                0x00,
                &block(0x46, &[(0x01, 0x01), (0x02, 0x48)]),
            ),
            dump(0x41, 0x10, 0x00, &block(0x67, &[(0x00, 1), (0x28, 0x7F)])),
            dump(0x0F, 0x52, 0x00, &[]),
        ],
    );
    assert_eq!(reader.finished(), 1);
    assert_eq!(reader.accepted(), 6);

    let sound = reader.into_sound(&device);
    let part = sound.part(1).expect("part 1 was read");
    assert_eq!(part.engine(), awm2::ENGINE_ID);
    assert_eq!(part.name(), Some("CFX Concert"));
    assert_eq!(part.value(part::VOLUME, None), Some(90));
    assert_eq!(part.value(part::FILTER_CUTOFF, None), Some(-16));
    assert_eq!(part.value(awm2::ELEMENT_SWITCH, Some(1)), Some(1));
    assert_eq!(part.value(awm2::ELEMENT_LEVEL, Some(1)), Some(80));
    assert_eq!(part.value(awm2::FILTER_CUTOFF, Some(1)), Some(200));
    assert_eq!(part.value(awm2::ELEMENT_SWITCH, Some(2)), Some(1));
    assert_eq!(part.value(awm2::ELEMENT_LEVEL, Some(2)), Some(127));
    assert_eq!(part.value(awm2::ELEMENT_PAN, Some(1)), Some(0));
    assert!(sound.part(2).is_none());
}

#[test]
fn reads_fm_x_and_drum_parts() {
    let device = ModxPlus::new();
    let sound = read(
        &device,
        &[
            dump(
                0x48,
                0x01,
                0x00,
                &block(0x56, &[(0x4F, 0x04), (0x50, 0x07)]),
            ),
            dump(
                0x49,
                0x21,
                0x00,
                &block(0x2F, &[(0x1A, 0x63), (0x06, 0x00)]),
            ),
            dump(
                0x31,
                0x02,
                0x00,
                &part_block("Drums", &[(0x24, 110), (0x2D, 0x58)]),
            ),
            dump(0x52, 0x24, 0x00, &block(0x2F, &[])),
        ],
    )
    .into_sound(&device);

    let fm = sound.part(2).expect("FM-X part");
    assert_eq!(fm.engine(), fm_x::ENGINE_ID);
    assert_eq!(fm.value(fm_x::ALGORITHM, None), Some(5));
    assert_eq!(fm.value(fm_x::FEEDBACK_LEVEL, None), Some(7));
    assert_eq!(fm.value(fm_x::OPERATOR_LEVEL, Some(3)), Some(99));
    assert_eq!(fm.value(fm_x::DETUNE, Some(3)), Some(-15));

    let drums = sound.part(3).expect("drum part");
    assert_eq!(drums.engine(), drum::ENGINE_ID);
    assert_eq!(drums.name(), Some("Drums"));
    assert_eq!(drums.value(part::VOLUME, None), Some(110));
    assert_eq!(drums.value(part::NOTE_SHIFT, None), None);
}

#[test]
fn out_of_range_raw_values_are_skipped() {
    let device = ModxPlus::new();
    let sound = read(
        &device,
        &[
            dump(0x41, 0x00, 0x00, &block(0x67, &[(0x49, 0x05)])),
            dump(0x31, 0x00, 0x00, &part_block("", &[(0x25, 0x00)])),
        ],
    )
    .into_sound(&device);
    let part = sound.part(1).unwrap();
    assert_eq!(part.name(), None);
    assert_eq!(part.value(awm2::COARSE_TUNE, Some(1)), Some(0));
    assert_eq!(part.value(part::PAN, None), Some(0));
}

#[test]
fn parameter_changes_from_the_keyboard_are_decoded() {
    let device = ModxPlus::new();
    let awm2_engine = device.engine(awm2::ENGINE_ID).unwrap();
    let message = awm2_engine
        .encode(&ParameterChange::new(
            awm2::FILTER_CUTOFF,
            Location::element(4, 3),
            250,
        ))
        .unwrap();
    assert_eq!(
        device.decode(message.as_bytes()),
        Received::Part(PartData {
            part: 4,
            engine: None,
            name: None,
            values: vec![ParameterChange::new(
                awm2::FILTER_CUTOFF,
                Location::element(4, 3),
                250
            )],
        })
    );

    let volume = awm2_engine
        .encode(&ParameterChange::new(part::VOLUME, Location::part(9), 12))
        .unwrap();
    assert_eq!(volume.to_hex(), "F0 43 10 7F 1C 07 31 08 24 0C F7");
    let Received::Part(data) = device.decode(volume.as_bytes()) else {
        panic!("volume change should decode");
    };
    assert_eq!(
        data.values,
        [ParameterChange::new(part::VOLUME, Location::part(9), 12)]
    );
}

#[test]
fn unrelated_messages_are_ignored() {
    let device = ModxPlus::new();
    let mut corrupted = dump(0x41, 0x00, 0x00, &block(0x67, &[]));
    let checksum = corrupted.len() - 2;
    corrupted[checksum] ^= 0x01;
    let cases = [
        vec![0x90, 0x3C, 0x40],
        vec![0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7],
        corrupted,
        dump(0x30, 0x40, 0x00, &[0x00]),
        dump(0x0E, 0x25, 0x00, &[]),
        yamaha::parameter_change(
            DeviceNumber::default(),
            MODEL,
            Address::new(0x00, 0x00, 0x00).unwrap(),
            &[0x7F],
        )
        .unwrap()
        .into_bytes(),
    ];
    for bytes in cases {
        assert_eq!(device.decode(&bytes), Received::Ignored, "{bytes:02X?}");
    }
    let mut reader = SoundReader::new();
    assert!(!reader.accept(Received::Ignored));
    assert_eq!(reader.accepted(), 0);
}

#[test]
fn part_parameters_encode_to_the_performance_part_table() {
    let device = ModxPlus::new();
    let engine = device.engine(awm2::ENGINE_ID).unwrap();
    let cases = [
        (part::VOLUME, 100, "31 00 24 64"),
        (part::PAN, -63, "31 00 25 01"),
        (part::NOTE_SHIFT, -24, "31 00 2D 28"),
        (part::NOTE_SHIFT, 24, "31 00 2D 58"),
        (part::AEG_ATTACK_TIME, -64, "31 00 46 00"),
        (part::AEG_DECAY_TIME, 0, "31 00 47 40"),
        (part::AEG_RELEASE_TIME, 63, "31 00 49 7F"),
        (part::FILTER_CUTOFF, 10, "31 00 50 4A"),
        (part::FILTER_RESONANCE, -10, "31 00 51 36"),
    ];
    for (id, value, payload) in cases {
        let message = engine
            .encode(&ParameterChange::new(id, Location::part(1), value))
            .unwrap();
        assert_eq!(
            message.to_hex(),
            format!("F0 43 10 7F 1C 07 {payload} F7"),
            "{id}"
        );
    }
}

#[test]
fn drum_parts_expose_only_part_parameters() {
    let device = ModxPlus::new();
    let drum_engine = device.engine(drum::ENGINE_ID).unwrap();
    assert!(drum_engine.parameter(part::NOTE_SHIFT).is_none());
    assert!(drum_engine.parameter(part::VOLUME).is_some());
    assert_eq!(drum_engine.element_label(), "Key");
    assert!(!drum_engine.description().is_empty());
    let message = drum_engine
        .encode(&ParameterChange::new(part::VOLUME, Location::part(10), 64))
        .unwrap();
    assert_eq!(message.to_hex(), "F0 43 10 7F 1C 07 31 09 24 40 F7");
    let _ = modx_plus::DEVICE_ID;
}
