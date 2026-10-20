use super::{END, ProtocolError, START, Segment, SysExMessage};

pub const MANUFACTURER_ID: u8 = 0x43;

const BULK_DUMP: u8 = 0x00;
const PARAMETER_CHANGE: u8 = 0x10;
const DUMP_REQUEST: u8 = 0x20;
const MAX_DATA_SIZE: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DeviceNumber(u8);

impl DeviceNumber {
    pub const fn new(value: u8) -> Result<Self, ProtocolError> {
        if value <= 0x0F {
            Ok(Self(value))
        } else {
            Err(ProtocolError::DeviceNumberOutOfRange(value))
        }
    }

    pub const fn get(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModelId(&'static [u8]);

impl ModelId {
    pub const fn new(bytes: &'static [u8]) -> Self {
        assert!(!bytes.is_empty(), "a model ID needs at least one byte");
        let mut index = 0;
        while index < bytes.len() {
            assert!(bytes[index] <= 0x7F, "model ID bytes must be 7-bit");
            index += 1;
        }
        Self(bytes)
    }

    pub const fn as_bytes(self) -> &'static [u8] {
        self.0
    }

    fn split(self) -> (&'static [u8], u8) {
        let (last, group) = self.0.split_last().unwrap_or((&0, &[]));
        (group, *last)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Address {
    high: u8,
    mid: u8,
    low: u8,
}

impl Address {
    pub const fn new(high: u8, mid: u8, low: u8) -> Result<Self, ProtocolError> {
        if high > 0x7F {
            return Err(ProtocolError::InvalidAddressByte(high));
        }
        if mid > 0x7F {
            return Err(ProtocolError::InvalidAddressByte(mid));
        }
        if low > 0x7F {
            return Err(ProtocolError::InvalidAddressByte(low));
        }
        Ok(Self { high, mid, low })
    }

    pub const fn to_bytes(self) -> [u8; 3] {
        [self.high, self.mid, self.low]
    }

    pub const fn high(self) -> u8 {
        self.high
    }

    pub const fn mid(self) -> u8 {
        self.mid
    }

    pub const fn low(self) -> u8 {
        self.low
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Incoming {
    ParameterChange {
        device: DeviceNumber,
        address: Address,
        data: Vec<u8>,
    },
    BulkDump {
        device: DeviceNumber,
        address: Address,
        data: Vec<u8>,
    },
}

pub fn encode_data(value: u32, size: usize) -> Result<Vec<u8>, ProtocolError> {
    if size == 0 || size > MAX_DATA_SIZE {
        return Err(ProtocolError::UnsupportedDataSize(size));
    }
    let bits = 7 * u32::try_from(size).map_err(|_| ProtocolError::UnsupportedDataSize(size))?;
    if value >> bits != 0 {
        return Err(ProtocolError::ValueTooLarge { value, size });
    }
    Ok((0..size)
        .rev()
        .map(|index| {
            let shifted = value >> (7 * index);
            u8::try_from(shifted & 0x7F).unwrap_or_default()
        })
        .collect())
}

pub fn decode_data(data: &[u8]) -> Result<u32, ProtocolError> {
    if data.is_empty() || data.len() > MAX_DATA_SIZE {
        return Err(ProtocolError::UnsupportedDataSize(data.len()));
    }
    data.iter().try_fold(0, |value, &byte| {
        if byte > 0x7F {
            Err(ProtocolError::InvalidDataByte(byte))
        } else {
            Ok((value << 7) | u32::from(byte))
        }
    })
}

pub fn dump_request(
    device: DeviceNumber,
    model: ModelId,
    address: Address,
) -> Result<SysExMessage, ProtocolError> {
    let model = model.as_bytes();
    let header_len = 3 + model.len();
    let mut bytes = Vec::with_capacity(header_len + 4);
    bytes.extend_from_slice(&[START, MANUFACTURER_ID, DUMP_REQUEST | device.get()]);
    bytes.extend_from_slice(model);
    bytes.extend_from_slice(&address.to_bytes());
    bytes.push(END);
    let segments = vec![
        Segment {
            label: "Header",
            range: 0..header_len,
        },
        Segment {
            label: "Address",
            range: header_len..header_len + 3,
        },
        Segment {
            label: "End",
            range: header_len + 3..header_len + 4,
        },
    ];
    Ok(SysExMessage::with_segments(bytes, segments)?)
}

pub fn bulk_dump(
    device: DeviceNumber,
    model: ModelId,
    address: Address,
    data: &[u8],
) -> Result<SysExMessage, ProtocolError> {
    let (group, model) = model.split();
    let mut counted = vec![model];
    counted.extend_from_slice(&address.to_bytes());
    counted.extend_from_slice(data);
    let count = u16::try_from(counted.len())
        .ok()
        .filter(|count| *count < 1 << 14)
        .ok_or(ProtocolError::UnsupportedDataSize(data.len()))?;
    let count_bytes = [
        u8::try_from(count >> 7).unwrap_or_default(),
        u8::try_from(count & 0x7F).unwrap_or_default(),
    ];
    let checksum = checksum(&counted);

    let mut bytes = vec![START, MANUFACTURER_ID, BULK_DUMP | device.get()];
    bytes.extend_from_slice(group);
    bytes.extend_from_slice(&count_bytes);
    bytes.push(model);
    bytes.extend_from_slice(&address.to_bytes());
    bytes.extend_from_slice(data);
    bytes.push(checksum);
    bytes.push(END);
    Ok(SysExMessage::new(bytes)?)
}

pub fn parse(bytes: &[u8], model: ModelId) -> Result<Incoming, ProtocolError> {
    let message = SysExMessage::new(bytes.to_vec())?;
    let bytes = message.as_bytes();
    let body = &bytes[1..bytes.len() - 1];
    let [MANUFACTURER_ID, status, rest @ ..] = body else {
        return Err(ProtocolError::Unrecognized);
    };
    let device = DeviceNumber::new(status & 0x0F)?;
    let (group, model_byte) = model.split();
    let rest = rest
        .strip_prefix(group)
        .ok_or(ProtocolError::Unrecognized)?;
    match status & 0x70 {
        PARAMETER_CHANGE => {
            let [found, high, mid, low, data @ ..] = rest else {
                return Err(ProtocolError::Unrecognized);
            };
            if *found != model_byte || data.is_empty() {
                return Err(ProtocolError::Unrecognized);
            }
            Ok(Incoming::ParameterChange {
                device,
                address: Address::new(*high, *mid, *low)?,
                data: data.to_vec(),
            })
        }
        BULK_DUMP => {
            let [count_high, count_low, found, high, mid, low, data @ .., sum] = rest else {
                return Err(ProtocolError::Unrecognized);
            };
            if *found != model_byte {
                return Err(ProtocolError::Unrecognized);
            }
            let count = (usize::from(*count_high) << 7) | usize::from(*count_low);
            let framed = data.len() + 4;
            if count != data.len() && count != framed {
                return Err(ProtocolError::ByteCountMismatch {
                    declared: count,
                    actual: data.len(),
                });
            }
            let payload: u32 = [*high, *mid, *low, *sum]
                .iter()
                .chain(data)
                .map(|byte| u32::from(*byte))
                .sum();
            let counted = payload + u32::from(model_byte);
            let documented = payload + u32::from(*count_high) + u32::from(*count_low);
            if counted & 0x7F != 0 && documented & 0x7F != 0 {
                return Err(ProtocolError::ChecksumMismatch);
            }
            Ok(Incoming::BulkDump {
                device,
                address: Address::new(*high, *mid, *low)?,
                data: data.to_vec(),
            })
        }
        _ => Err(ProtocolError::Unrecognized),
    }
}

fn checksum(bytes: &[u8]) -> u8 {
    let sum = bytes.iter().map(|byte| u32::from(*byte)).sum::<u32>();
    u8::try_from((0x80 - (sum & 0x7F)) & 0x7F).unwrap_or_default()
}

pub fn parameter_change(
    device: DeviceNumber,
    model: ModelId,
    address: Address,
    data: &[u8],
) -> Result<SysExMessage, ProtocolError> {
    if data.is_empty() || data.len() > MAX_DATA_SIZE {
        return Err(ProtocolError::UnsupportedDataSize(data.len()));
    }
    let model = model.as_bytes();
    let header_len = 3 + model.len();
    let address_end = header_len + 3;
    let data_end = address_end + data.len();

    let mut bytes = Vec::with_capacity(data_end + 1);
    bytes.extend_from_slice(&[START, MANUFACTURER_ID, PARAMETER_CHANGE | device.get()]);
    bytes.extend_from_slice(model);
    bytes.extend_from_slice(&address.to_bytes());
    bytes.extend_from_slice(data);
    bytes.push(END);

    let segments = vec![
        Segment {
            label: "Header",
            range: 0..header_len,
        },
        Segment {
            label: "Address",
            range: header_len..address_end,
        },
        Segment {
            label: "Data",
            range: address_end..data_end,
        },
        Segment {
            label: "End",
            range: data_end..data_end + 1,
        },
    ];
    Ok(SysExMessage::with_segments(bytes, segments)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODEL: ModelId = ModelId::new(&[0x7F, 0x1C, 0x07]);

    #[test]
    fn encodes_single_byte_data() {
        assert_eq!(encode_data(0x7F, 1), Ok(vec![0x7F]));
        assert_eq!(
            encode_data(0x80, 1),
            Err(ProtocolError::ValueTooLarge {
                value: 0x80,
                size: 1
            })
        );
    }

    #[test]
    fn encodes_two_byte_data_as_seven_bit_groups() {
        assert_eq!(encode_data(160, 2), Ok(vec![0x01, 0x20]));
        assert_eq!(encode_data(255, 2), Ok(vec![0x01, 0x7F]));
        assert_eq!(encode_data(0x3FFF, 2), Ok(vec![0x7F, 0x7F]));
    }

    #[test]
    fn rejects_unsupported_sizes() {
        assert_eq!(
            encode_data(0, 0),
            Err(ProtocolError::UnsupportedDataSize(0))
        );
        assert_eq!(
            encode_data(0, 5),
            Err(ProtocolError::UnsupportedDataSize(5))
        );
    }

    #[test]
    fn rejects_invalid_device_numbers_and_addresses() {
        assert_eq!(
            DeviceNumber::new(16),
            Err(ProtocolError::DeviceNumberOutOfRange(16))
        );
        assert_eq!(
            Address::new(0x41, 0x80, 0x00),
            Err(ProtocolError::InvalidAddressByte(0x80))
        );
    }

    #[test]
    fn builds_parameter_change_with_segments() {
        let device = DeviceNumber::new(3).unwrap();
        let address = Address::new(0x41, 0x00, 0x28).unwrap();
        let message = parameter_change(device, MODEL, address, &[0x64]).unwrap();
        assert_eq!(
            message.as_bytes(),
            &[
                0xF0, 0x43, 0x13, 0x7F, 0x1C, 0x07, 0x41, 0x00, 0x28, 0x64, 0xF7
            ]
        );
        let labels: Vec<_> = message
            .segments()
            .iter()
            .map(|segment| (segment.label, segment.range.clone()))
            .collect();
        assert_eq!(
            labels,
            vec![
                ("Header", 0..6),
                ("Address", 6..9),
                ("Data", 9..10),
                ("End", 10..11)
            ]
        );
    }

    #[test]
    fn model_ids_and_device_numbers_round_trip() {
        let model = ModelId::new(&[0x7F, 0x1C, 0x07]);
        assert_eq!(model.as_bytes(), &[0x7F, 0x1C, 0x07]);
        assert_eq!(DeviceNumber::new(15).map(DeviceNumber::get), Ok(15));
    }

    #[test]
    #[should_panic(expected = "7-bit")]
    fn model_ids_must_be_seven_bit() {
        let _ = ModelId::new(&[0x80]);
    }

    #[test]
    fn every_address_byte_is_checked() {
        assert_eq!(
            Address::new(0x80, 0x00, 0x00),
            Err(ProtocolError::InvalidAddressByte(0x80))
        );
        assert_eq!(
            Address::new(0x00, 0x00, 0xFF),
            Err(ProtocolError::InvalidAddressByte(0xFF))
        );
    }

    #[test]
    fn parameter_changes_need_data() {
        let address = Address::new(0x41, 0x00, 0x28).unwrap();
        assert_eq!(
            parameter_change(DeviceNumber::default(), MODEL, address, &[]),
            Err(ProtocolError::UnsupportedDataSize(0))
        );
    }

    #[test]
    fn bulk_dumps_round_trip_with_their_checksum() {
        let address = Address::new(0x41, 0x00, 0x00).unwrap();
        let message = bulk_dump(DeviceNumber::default(), MODEL, address, &[0x01, 0x02]).unwrap();
        assert_eq!(
            message.to_hex(),
            "F0 43 00 7F 1C 00 06 07 41 00 00 01 02 35 F7"
        );
        assert_eq!(
            parse(message.as_bytes(), MODEL),
            Ok(Incoming::BulkDump {
                device: DeviceNumber::default(),
                address,
                data: vec![0x01, 0x02],
            })
        );
    }

    #[test]
    fn bulk_dumps_with_a_wrong_byte_count_are_rejected() {
        let mut bytes = bulk_dump(
            DeviceNumber::default(),
            MODEL,
            Address::new(0x41, 0x00, 0x00).unwrap(),
            &[0x01, 0x02],
        )
        .unwrap()
        .into_bytes();
        bytes[6] = 0x03;
        assert_eq!(
            parse(&bytes, MODEL),
            Err(ProtocolError::ByteCountMismatch {
                declared: 3,
                actual: 2
            })
        );
    }

    #[test]
    fn foreign_and_truncated_messages_are_unrecognized() {
        let cases: [&[u8]; 6] = [
            &[0xF0, 0x41, 0x10, 0x00, 0xF7],
            &[
                0xF0, 0x43, 0x10, 0x7F, 0x1D, 0x07, 0x41, 0x00, 0x00, 0x01, 0xF7,
            ],
            &[
                0xF0, 0x43, 0x10, 0x7F, 0x1C, 0x02, 0x41, 0x00, 0x00, 0x01, 0xF7,
            ],
            &[0xF0, 0x43, 0x10, 0x7F, 0x1C, 0x07, 0x41, 0x00, 0x00, 0xF7],
            &[
                0xF0, 0x43, 0x00, 0x7F, 0x1C, 0x00, 0x00, 0x02, 0x41, 0x00, 0x00, 0x3F, 0xF7,
            ],
            &[0xF0, 0x43, 0x30, 0x7F, 0x1C, 0x07, 0x41, 0x00, 0x00, 0xF7],
        ];
        for bytes in cases {
            assert_eq!(
                parse(bytes, MODEL),
                Err(ProtocolError::Unrecognized),
                "{bytes:02X?}"
            );
        }
        assert!(parse(&[0x90, 0x3C, 0x40], MODEL).is_err());
    }

    #[test]
    fn parameter_changes_are_parsed() {
        let address = Address::new(0x31, 0x02, 0x24).unwrap();
        let device = DeviceNumber::new(4).unwrap();
        let message = parameter_change(device, MODEL, address, &[0x50]).unwrap();
        assert_eq!(
            parse(message.as_bytes(), MODEL),
            Ok(Incoming::ParameterChange {
                device,
                address,
                data: vec![0x50],
            })
        );
        assert_eq!(
            (address.high(), address.mid(), address.low()),
            (0x31, 0x02, 0x24)
        );
    }

    #[test]
    fn data_bytes_decode_to_values() {
        assert_eq!(decode_data(&[0x01, 0x48]), Ok(200));
        assert_eq!(decode_data(&[0x7F]), Ok(127));
        assert_eq!(decode_data(&[]), Err(ProtocolError::UnsupportedDataSize(0)));
        assert_eq!(
            decode_data(&[0x80]),
            Err(ProtocolError::InvalidDataByte(0x80))
        );
    }

    #[test]
    fn dump_requests_address_a_block() {
        let address = Address::new(0x0E, 0x52, 0x03).unwrap();
        let message = dump_request(DeviceNumber::default(), MODEL, address).unwrap();
        assert_eq!(message.to_hex(), "F0 43 20 7F 1C 07 0E 52 03 F7");
        assert_eq!(message.segments().len(), 3);
    }

    #[test]
    fn oversized_bulk_dumps_are_rejected() {
        let address = Address::new(0x41, 0x00, 0x00).unwrap();
        assert_eq!(
            bulk_dump(DeviceNumber::default(), MODEL, address, &vec![0; 1 << 14]),
            Err(ProtocolError::UnsupportedDataSize(1 << 14))
        );
    }

    fn documented_dump(count: [u8; 2], data: &[u8]) -> Vec<u8> {
        let mut body = count.to_vec();
        body.extend_from_slice(&[0x0E, 0x52, 0x00]);
        body.extend_from_slice(data);
        let sum: u32 = body.iter().map(|byte| u32::from(*byte)).sum();
        let checksum = u8::try_from((0x80 - (sum & 0x7F)) & 0x7F).unwrap();
        let mut bytes = vec![0xF0, 0x43, 0x00, 0x7F, 0x1C, count[0], count[1], 0x07];
        bytes.extend_from_slice(&[0x0E, 0x52, 0x00]);
        bytes.extend_from_slice(data);
        bytes.extend_from_slice(&[checksum, 0xF7]);
        bytes
    }

    #[test]
    fn reads_a_bulk_header_captured_from_a_modx_plus() {
        let captured = [
            0xF0, 0x43, 0x00, 0x7F, 0x1C, 0x00, 0x04, 0x07, 0x0E, 0x52, 0x00, 0x19, 0xF7,
        ];
        assert_eq!(
            parse(&captured, MODEL),
            Ok(Incoming::BulkDump {
                device: DeviceNumber::default(),
                address: Address::new(0x0E, 0x52, 0x00).unwrap(),
                data: Vec::new(),
            })
        );
        let rebuilt = bulk_dump(
            DeviceNumber::default(),
            MODEL,
            Address::new(0x0E, 0x52, 0x00).unwrap(),
            &[],
        )
        .unwrap();
        assert_eq!(rebuilt.as_bytes(), captured);
    }

    #[test]
    fn the_documented_checksum_variant_is_also_accepted() {
        for (count, data) in [([0x00, 0x00], &[][..]), ([0x00, 0x02], &[0x01, 0x02][..])] {
            assert!(
                matches!(
                    parse(&documented_dump(count, data), MODEL),
                    Ok(Incoming::BulkDump { .. })
                ),
                "{count:02X?}"
            );
        }
        let mut wrong = documented_dump([0x00, 0x00], &[]);
        let index = wrong.len() - 2;
        wrong[index] = wrong[index].wrapping_add(3) & 0x7F;
        assert_eq!(parse(&wrong, MODEL), Err(ProtocolError::ChecksumMismatch));
    }
}
