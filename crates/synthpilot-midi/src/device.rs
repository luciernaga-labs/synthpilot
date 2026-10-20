use std::sync::mpsc::{self, Receiver, Sender};

use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use synthpilot_core::protocol::SysExMessage;

use crate::{Connection, MidiError, Packet, Port};

const CLIENT_NAME: &str = "SynthPilot";
const OUTPUT_NAME: &str = "SynthPilot Output";
const INPUT_NAME: &str = "SynthPilot Input";

pub fn output_ports() -> Result<Vec<Port>, MidiError> {
    let output = output_client()?;
    Ok(output
        .ports()
        .iter()
        .filter_map(|port| {
            let name = output.port_name(port).ok()?;
            Some(Port {
                id: port.id(),
                name,
            })
        })
        .collect())
}

fn output_client() -> Result<MidiOutput, MidiError> {
    MidiOutput::new(CLIENT_NAME).map_err(|error| MidiError::Unavailable(error.to_string()))
}

fn open_inputs(sender: &Sender<Packet>) -> Vec<MidiInputConnection<()>> {
    let count = MidiInput::new(CLIENT_NAME).map_or(0, |input| input.port_count());
    (0..count)
        .filter_map(|index| {
            let input = MidiInput::new(CLIENT_NAME).ok()?;
            let port = input.ports().into_iter().nth(index)?;
            let name = input.port_name(&port).ok()?;
            let sender = sender.clone();
            input
                .connect(
                    &port,
                    INPUT_NAME,
                    move |_, bytes, ()| {
                        let _ = sender.send(Packet {
                            port: name.clone(),
                            bytes: bytes.to_vec(),
                        });
                    },
                    (),
                )
                .ok()
        })
        .collect()
}

pub struct MidiConnection {
    port: Port,
    output: MidiOutputConnection,
    inputs: Vec<MidiInputConnection<()>>,
    packets: Receiver<Packet>,
}

impl MidiConnection {
    pub fn open(port: &Port) -> Result<Self, MidiError> {
        let output = output_client()?;
        let midi_port = output
            .find_port_by_id(&port.id)
            .ok_or_else(|| MidiError::PortNotFound(port.name.clone()))?;
        let connection =
            output
                .connect(&midi_port, OUTPUT_NAME)
                .map_err(|error| MidiError::Connect {
                    port: port.name.clone(),
                    reason: error.to_string(),
                })?;
        let (sender, packets) = mpsc::channel();
        Ok(Self {
            port: port.clone(),
            output: connection,
            inputs: open_inputs(&sender),
            packets,
        })
    }
}

impl Connection for MidiConnection {
    fn port(&self) -> &Port {
        &self.port
    }

    fn can_receive(&self) -> bool {
        !self.inputs.is_empty()
    }

    fn send(&mut self, message: &SysExMessage) -> Result<(), MidiError> {
        self.output
            .send(message.as_bytes())
            .map_err(|error| MidiError::Send {
                port: self.port.name.clone(),
                reason: error.to_string(),
            })
    }

    fn receive(&mut self) -> Vec<Packet> {
        self.packets.try_iter().collect()
    }
}
