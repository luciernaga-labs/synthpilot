use std::fmt;
use std::ops::Range;

use thiserror::Error;

pub const START: u8 = 0xF0;
pub const END: u8 = 0xF7;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SysExError {
    #[error("a SysEx message must start with F0 and end with F7")]
    MissingFraming,
    #[error("byte {byte:#04X} at position {position} is not a 7-bit data byte")]
    InvalidDataByte { position: usize, byte: u8 },
    #[error("segment {label:?} covers {start}..{end}, outside the {len}-byte message")]
    InvalidSegment {
        label: &'static str,
        start: usize,
        end: usize,
        len: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Segment {
    pub label: &'static str,
    pub range: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SysExMessage {
    bytes: Vec<u8>,
    segments: Vec<Segment>,
}

impl SysExMessage {
    pub fn new(bytes: Vec<u8>) -> Result<Self, SysExError> {
        Self::with_segments(bytes, Vec::new())
    }

    pub fn with_segments(bytes: Vec<u8>, segments: Vec<Segment>) -> Result<Self, SysExError> {
        let (Some(&START), Some(&END)) = (bytes.first(), bytes.last()) else {
            return Err(SysExError::MissingFraming);
        };
        if let Some((position, &byte)) = bytes[1..bytes.len() - 1]
            .iter()
            .enumerate()
            .find(|(_, byte)| **byte > 0x7F)
        {
            return Err(SysExError::InvalidDataByte {
                position: position + 1,
                byte,
            });
        }
        if let Some(segment) = segments.iter().find(|segment| {
            segment.range.start > segment.range.end || segment.range.end > bytes.len()
        }) {
            return Err(SysExError::InvalidSegment {
                label: segment.label,
                start: segment.range.start,
                end: segment.range.end,
                len: bytes.len(),
            });
        }
        Ok(Self { bytes, segments })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn to_hex(&self) -> String {
        hex(&self.bytes)
    }
}

impl AsRef<[u8]> for SysExMessage {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl fmt::Display for SysExMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

pub fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_missing_framing() {
        assert_eq!(
            SysExMessage::new(vec![0x43, 0x10]),
            Err(SysExError::MissingFraming)
        );
        assert_eq!(SysExMessage::new(vec![]), Err(SysExError::MissingFraming));
    }

    #[test]
    fn rejects_non_seven_bit_payload() {
        assert_eq!(
            SysExMessage::new(vec![START, 0x43, 0x80, END]),
            Err(SysExError::InvalidDataByte {
                position: 2,
                byte: 0x80
            })
        );
    }

    #[test]
    fn formats_as_uppercase_hex() {
        let message = SysExMessage::new(vec![START, 0x43, 0x0A, END]).unwrap();
        assert_eq!(message.to_string(), "F0 43 0A F7");
    }

    #[test]
    fn rejects_out_of_bounds_segments() {
        let segment = Segment {
            label: "Data",
            range: 2..9,
        };
        assert!(matches!(
            SysExMessage::with_segments(vec![START, 0x01, END], vec![segment]),
            Err(SysExError::InvalidSegment { .. })
        ));
    }

    #[test]
    fn exposes_its_bytes() {
        let message = SysExMessage::new(vec![START, 0x43, END]).unwrap();
        assert_eq!(message.len(), 3);
        assert!(!message.is_empty());
        assert_eq!(message.as_ref(), &[START, 0x43, END]);
        assert_eq!(message.segments(), []);
        assert_eq!(message.into_bytes(), vec![START, 0x43, END]);
    }
}
