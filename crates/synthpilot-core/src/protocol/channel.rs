use std::fmt;

use super::hex;

/// A MIDI channel voice message, such as a Bank Select or a Program Change.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChannelMessage {
    bytes: Vec<u8>,
}

impl ChannelMessage {
    /// A Control Change on `channel` (0–15).
    pub fn control_change(channel: u8, control: u8, value: u8) -> Option<Self> {
        (channel <= 0x0F && control <= 0x7F && value <= 0x7F).then(|| Self {
            bytes: vec![0xB0 | channel, control, value],
        })
    }

    /// A Program Change on `channel` (0–15), with `program` 0–127.
    pub fn program_change(channel: u8, program: u8) -> Option<Self> {
        (channel <= 0x0F && program <= 0x7F).then(|| Self {
            bytes: vec![0xC0 | channel, program],
        })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn to_hex(&self) -> String {
        hex(&self.bytes)
    }
}

impl fmt::Display for ChannelMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_channel_messages() {
        let bank = ChannelMessage::control_change(6, 0x00, 0x3F).unwrap();
        assert_eq!(bank.to_hex(), "B6 00 3F");
        assert_eq!(bank.to_string(), "B6 00 3F");
        assert_eq!(
            ChannelMessage::program_change(15, 127).unwrap().as_bytes(),
            [0xCF, 0x7F]
        );
        assert!(ChannelMessage::control_change(16, 0, 0).is_none());
        assert!(ChannelMessage::control_change(0, 0x80, 0).is_none());
        assert!(ChannelMessage::control_change(0, 0, 0x80).is_none());
        assert!(ChannelMessage::program_change(0, 0x80).is_none());
    }
}
