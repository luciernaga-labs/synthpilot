mod device;

use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};

pub use device::{MidiConnection, output_ports};
use synthpilot_core::protocol::SysExMessage;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MidiError {
    #[error(
        "Your computer's MIDI system isn't responding. Unplug the keyboard's USB cable, plug it back in and try again. If that doesn't help, restart the computer. Details: {0}"
    )]
    Unavailable(String),
    #[error(
        "{0} is no longer available. Check that the keyboard is on and its USB cable is plugged in, then choose the port again."
    )]
    PortNotFound(String),
    #[error(
        "Couldn't connect to {port}. Another app may be using it (a DAW or another editor). Close that app and try again. Details: {reason}"
    )]
    Connect { port: String, reason: String },
    #[error(
        "Lost the connection to {port}. Check the USB cable and that the keyboard is on, then connect again. Changes you already made are still on the keyboard. Details: {reason}"
    )]
    Send { port: String, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Port {
    pub id: String,
    pub name: String,
}

impl fmt::Display for Port {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Packet {
    pub port: String,
    pub bytes: Vec<u8>,
}

pub trait Connection {
    fn port(&self) -> &Port;

    fn can_receive(&self) -> bool;

    fn send(&mut self, message: &SysExMessage) -> Result<(), MidiError>;

    fn receive(&mut self) -> Vec<Packet>;
}

#[derive(Debug, Clone)]
pub struct RecordingConnection {
    port: Port,
    sent: Arc<Mutex<Vec<SysExMessage>>>,
    incoming: Arc<Mutex<VecDeque<Packet>>>,
}

impl RecordingConnection {
    pub fn new(name: &str) -> Self {
        Self {
            port: Port {
                id: name.to_owned(),
                name: name.to_owned(),
            },
            sent: Arc::default(),
            incoming: Arc::default(),
        }
    }

    pub fn deliver(&self, bytes: Vec<u8>) {
        let port = self.port.name.clone();
        self.deliver_from(&port, bytes);
    }

    pub fn deliver_from(&self, port: &str, bytes: Vec<u8>) {
        self.incoming
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push_back(Packet {
                port: port.to_owned(),
                bytes,
            });
    }

    pub fn sent(&self) -> Vec<SysExMessage> {
        self.sent
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Connection for RecordingConnection {
    fn port(&self) -> &Port {
        &self.port
    }

    fn can_receive(&self) -> bool {
        true
    }

    fn receive(&mut self) -> Vec<Packet> {
        self.incoming
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .drain(..)
            .collect()
    }

    fn send(&mut self, message: &SysExMessage) -> Result<(), MidiError> {
        self.sent
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(message.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_connection_keeps_messages_in_order() {
        let first = SysExMessage::new(vec![0xF0, 0x01, 0xF7]).unwrap();
        let second = SysExMessage::new(vec![0xF0, 0x02, 0xF7]).unwrap();
        let log = RecordingConnection::new("Test");
        let mut output = log.clone();
        output.send(&first).unwrap();
        output.send(&second).unwrap();
        assert_eq!(log.sent(), [first, second]);
        assert_eq!(output.port().to_string(), "Test");
        assert!(output.can_receive());
    }

    #[test]
    fn delivered_messages_are_received_once() {
        let keyboard = RecordingConnection::new("Test");
        let mut connection = keyboard.clone();
        keyboard.deliver(vec![0xF0, 0x01, 0xF7]);
        keyboard.deliver_from("Other", vec![0x90, 0x3C, 0x40]);
        assert_eq!(
            connection.receive(),
            [
                Packet {
                    port: "Test".to_owned(),
                    bytes: vec![0xF0, 0x01, 0xF7]
                },
                Packet {
                    port: "Other".to_owned(),
                    bytes: vec![0x90, 0x3C, 0x40]
                }
            ]
        );
        assert_eq!(connection.receive(), [] as [Packet; 0]);
    }
}
