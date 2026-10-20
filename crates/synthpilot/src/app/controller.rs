use std::time::{Duration, Instant};

use synthpilot_core::devices;
use synthpilot_core::domain::{Received, SoundReader};
use synthpilot_core::protocol::hex;

use super::midi::{MidiAction, MidiLink};
use super::state::{Action, Workspace};

const QUIET_PERIOD: Duration = Duration::from_millis(1500);
const REPLY_TIMEOUT: Duration = Duration::from_secs(4);

const NO_INPUT: &str = "SynthPilot can send to the keyboard but can't hear back from it, so it \
                        can't read the Performance. Close other apps that use the keyboard (a DAW \
                        or another editor), then select Disconnect and connect again.";
const NO_REPLY: &str = "The keyboard didn't answer. On the MODX+, go to Utility > Settings > \
                        MIDI I/O and set MIDI IN/OUT to USB, Device Number to All and Bulk \
                        Receive to On. Then select Read from keyboard.";
const OTHER_TRAFFIC: &str = "The keyboard is sending MIDI (for example, notes) but not its \
                             Performance data. On the MODX+, go to Utility > Settings > MIDI I/O \
                             and set Device Number to All and Bulk Receive to On. Then select Read \
                             from keyboard.";
const SHOWN_BYTES: usize = 24;

struct Reading {
    reader: SoundReader,
    requested: usize,
    started: Instant,
    last: Instant,
    unread: usize,
    first_unread: Option<Vec<u8>>,
    other: usize,
    ports: Vec<String>,
}

impl Reading {
    fn unreadable(&self) -> String {
        let Some(bytes) = self.first_unread.as_deref() else {
            if self.other > 0 {
                return OTHER_TRAFFIC.to_owned();
            }
            return NO_REPLY.to_owned();
        };
        let shown = hex(&bytes[..bytes.len().min(SHOWN_BYTES)]);
        let more = if bytes.len() > SHOWN_BYTES {
            " …"
        } else {
            ""
        };
        format!(
            "The keyboard answered, but SynthPilot couldn't understand the reply. Check that the \
             selected synthesizer matches your keyboard. If it keeps happening, report it with \
             these bytes ({} messages, first one {} bytes): {shown}{more}",
            self.unread,
            bytes.len(),
        )
    }
}

pub(crate) struct Controller {
    workspace: Option<Workspace>,
    midi: MidiLink,
    reading: Option<Reading>,
    status: Option<String>,
    error: Option<String>,
}

impl Controller {
    pub(crate) const fn new(midi: MidiLink) -> Self {
        Self {
            workspace: None,
            midi,
            reading: None,
            status: None,
            error: None,
        }
    }

    pub(crate) const fn workspace(&self) -> Option<&Workspace> {
        self.workspace.as_ref()
    }

    pub(crate) const fn midi(&self) -> &MidiLink {
        &self.midi
    }

    pub(crate) fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub(crate) fn reading(&self) -> Option<(usize, usize)> {
        self.reading
            .as_ref()
            .map(|reading| (reading.reader.finished(), reading.requested))
    }

    pub(crate) fn dispatch(
        &mut self,
        actions: Vec<Action>,
        midi_actions: Vec<MidiAction>,
        now: Instant,
    ) {
        let was_live = self.midi.live_port().is_some();
        for action in midi_actions {
            self.midi.apply(action);
        }
        if !was_live && self.midi.live_port().is_some() {
            self.start_reading(now);
        }
        self.leave_if_disconnected();
        for action in actions {
            if action == Action::ReadFromKeyboard {
                self.start_reading(now);
                continue;
            }
            match (&mut self.workspace, action) {
                (None, Action::SelectDevice(id)) if self.midi.live_port().is_some() => {
                    self.workspace = devices::load(id).map(Workspace::new);
                    self.start_reading(now);
                }
                (Some(workspace), action) => {
                    let messages = workspace.apply(action);
                    self.midi.send(&messages);
                }
                (None, _) => {}
            }
        }
        self.leave_if_disconnected();
    }

    fn leave_if_disconnected(&mut self) {
        if self.midi.live_port().is_none() {
            self.workspace = None;
            self.reading = None;
            self.status = None;
        }
    }

    pub(crate) fn tick(&mut self, now: Instant) {
        self.midi.tick(now);
        self.leave_if_disconnected();
        let incoming = self.midi.receive();
        let Some(workspace) = self.workspace.as_mut() else {
            return;
        };
        for packet in incoming {
            let received = workspace.device().decode(&packet.bytes);
            match self.reading.as_mut() {
                Some(reading) => {
                    if reading.reader.accept(received) {
                        reading.last = now;
                        if !reading.ports.contains(&packet.port) {
                            reading.ports.push(packet.port);
                        }
                    } else if packet.bytes.first() == Some(&0xF0) {
                        reading.unread += 1;
                        reading.first_unread.get_or_insert(packet.bytes);
                    } else {
                        reading.other += 1;
                    }
                }
                None => {
                    if let Received::Part(data) = received {
                        workspace.receive(&data);
                    }
                }
            }
        }

        let Some(reading) = self.reading.as_ref() else {
            return;
        };
        let accepted = reading.reader.accepted();
        let complete = reading.reader.finished() >= reading.requested;
        let quiet = accepted > 0 && now.duration_since(reading.last) >= QUIET_PERIOD;
        let timed_out = accepted == 0 && now.duration_since(reading.started) >= REPLY_TIMEOUT;
        if complete || quiet {
            let Some(reading) = self.reading.take() else {
                return;
            };
            let live = self.midi.live_port().map(|port| port.name.clone());
            let elsewhere: Vec<_> = reading
                .ports
                .iter()
                .filter(|port| Some(*port) != live.as_ref())
                .cloned()
                .collect();
            let sound = reading.reader.into_sound(workspace.device());
            let parts = (1..=sound.part_count())
                .filter(|number| sound.part(*number).is_some())
                .count();
            workspace.load(sound);
            let status = match parts {
                0 => {
                    "The keyboard answered, but this Performance has no Parts SynthPilot can edit."
                        .to_owned()
                }
                1 => "Read 1 Part from the keyboard.".to_owned(),
                parts => format!("Read {parts} Parts from the keyboard."),
            };
            self.status = Some(if elsewhere.is_empty() {
                status
            } else {
                format!(
                    "{status} The keyboard answered on {}.",
                    elsewhere.join(", ")
                )
            });
        } else if timed_out {
            self.error = self.reading.take().map(|reading| reading.unreadable());
            self.status = None;
        }
    }

    fn start_reading(&mut self, now: Instant) {
        self.error = None;
        let Some(workspace) = self.workspace.as_ref() else {
            return;
        };
        if self.midi.live_port().is_none() {
            return;
        }
        if !self.midi.can_receive() {
            self.error = Some(NO_INPUT.to_owned());
            return;
        }
        let requests = workspace.device().read_requests();
        if requests.is_empty() {
            return;
        }
        self.reading = Some(Reading {
            reader: SoundReader::new(),
            requested: requests.len(),
            started: now,
            last: now,
            unread: 0,
            first_unread: None,
            other: 0,
            ports: Vec::new(),
        });
        self.status = Some("Reading the Performance from the keyboard…".to_owned());
        self.midi.send(&requests);
    }

    #[cfg(test)]
    fn connect(&mut self, connection: Box<dyn synthpilot_midi::Connection>) {
        self.midi.attach(connection);
    }
}

#[cfg(test)]
mod tests {
    use synthpilot_core::devices::modx_plus::{self, awm2, part};
    use synthpilot_core::domain::{DeviceProfile, Location, ParameterChange};
    use synthpilot_core::protocol::SysExMessage;
    use synthpilot_core::protocol::yamaha::{self, Address, DeviceNumber, ModelId};
    use synthpilot_midi::{Connection, MidiError, Port, RecordingConnection};

    use super::*;

    const MODEL: ModelId = ModelId::new(&[0x7F, 0x1C, 0x07]);

    fn dump(high: u8, mid: u8, low: u8, data: &[u8]) -> Vec<u8> {
        let address = Address::new(high, mid, low).unwrap();
        yamaha::bulk_dump(DeviceNumber::default(), MODEL, address, data)
            .unwrap()
            .into_bytes()
    }

    fn piano(keyboard: &RecordingConnection) {
        let mut part_block = vec![0x40; 0x79];
        part_block[..20].copy_from_slice(b"CFX Concert         ");
        let mut element = vec![0x40; 0x67];
        element[0x00] = 1;
        element[0x28] = 0x50;
        keyboard.deliver(dump(0x0E, 0x52, 0x00, &[]));
        keyboard.deliver(dump(0x31, 0x00, 0x00, &part_block));
        keyboard.deliver(dump(0x41, 0x00, 0x00, &element));
        keyboard.deliver(dump(0x0F, 0x52, 0x00, &[]));
    }

    fn connected(now: Instant) -> (Controller, RecordingConnection) {
        let keyboard = RecordingConnection::new("MODX Port 1");
        let mut controller = Controller::new(MidiLink::idle());
        controller.connect(Box::new(keyboard.clone()));
        controller.dispatch(
            vec![Action::SelectDevice(modx_plus::DEVICE_ID)],
            Vec::new(),
            now,
        );
        (controller, keyboard)
    }

    fn settle(controller: &mut Controller, now: Instant) {
        controller.tick(now);
        controller.tick(now + QUIET_PERIOD);
    }

    fn sent(keyboard: &RecordingConnection) -> Vec<String> {
        keyboard.sent().iter().map(SysExMessage::to_hex).collect()
    }

    #[test]
    fn connecting_asks_the_keyboard_for_its_sound() {
        let now = Instant::now();
        let (controller, keyboard) = connected(now);
        let requests = sent(&keyboard);
        assert_eq!(requests.len(), 17);
        assert_eq!(requests[0], "F0 43 20 7F 1C 07 0E 52 00 F7");
        assert_eq!(controller.reading(), Some((0, 17)));
        assert_eq!(
            controller.status(),
            Some("Reading the Performance from the keyboard…")
        );
    }

    #[test]
    fn replies_become_the_sound_being_edited() {
        let now = Instant::now();
        let (mut controller, keyboard) = connected(now);
        piano(&keyboard);
        for part in 1..16 {
            keyboard.deliver(dump(0x0F, 0x52, part, &[]));
        }
        let mut common = vec![0x00; 0x5E];
        common[..11].copy_from_slice(b"Grand Piano");
        keyboard.deliver(dump(0x30, 0x40, 0x00, &common));
        keyboard.deliver(dump(0x0F, 0x53, 0x00, &[]));
        controller.tick(now);

        assert!(controller.reading().is_none());
        assert_eq!(controller.status(), Some("Read 1 Part from the keyboard."));
        let workspace = controller.workspace().unwrap();
        assert_eq!(workspace.part(), 1);
        assert_eq!(workspace.sound().unwrap().name(), Some("Grand Piano"));
        let part = workspace.sound().unwrap().part(1).unwrap();
        assert_eq!(part.name(), Some("CFX Concert"));
        assert_eq!(part.value(awm2::ELEMENT_LEVEL, Some(1)), Some(80));
    }

    #[test]
    fn reading_finishes_after_a_quiet_period() {
        let now = Instant::now();
        let (mut controller, keyboard) = connected(now);
        piano(&keyboard);
        controller.tick(now);
        assert_eq!(controller.reading(), Some((1, 17)));
        controller.tick(now + QUIET_PERIOD);
        assert!(controller.reading().is_none());
        assert!(controller.workspace().unwrap().sound().is_some());
    }

    #[test]
    fn empty_replies_are_reported() {
        let now = Instant::now();
        let (mut controller, keyboard) = connected(now);
        keyboard.deliver(dump(0x0E, 0x52, 0x00, &[]));
        settle(&mut controller, now);
        assert_eq!(
            controller.status(),
            Some("The keyboard answered, but this Performance has no Parts SynthPilot can edit.")
        );
    }

    #[test]
    fn several_parts_are_counted() {
        let now = Instant::now();
        let (mut controller, keyboard) = connected(now);
        piano(&keyboard);
        keyboard.deliver(dump(0x48, 0x01, 0x00, &[0x40; 0x56]));
        settle(&mut controller, now);
        assert_eq!(controller.status(), Some("Read 2 Parts from the keyboard."));
    }

    #[test]
    fn silence_explains_what_to_check() {
        let now = Instant::now();
        let (mut controller, _keyboard) = connected(now);
        controller.tick(now + Duration::from_secs(1));
        assert!(controller.reading().is_some());
        controller.tick(now + REPLY_TIMEOUT);
        assert!(controller.reading().is_none());
        assert_eq!(controller.error(), Some(NO_REPLY));
    }

    #[test]
    fn replies_on_another_port_are_used_and_reported() {
        let now = Instant::now();
        let (mut controller, keyboard) = connected(now);
        let mut element = vec![0x40; 0x67];
        element[0x00] = 1;
        keyboard.deliver_from("MODX Port 3", dump(0x41, 0x00, 0x00, &element));
        settle(&mut controller, now);
        assert_eq!(
            controller.status(),
            Some("Read 1 Part from the keyboard. The keyboard answered on MODX Port 3.")
        );
    }

    #[test]
    fn other_midi_traffic_is_reported_when_no_dump_arrives() {
        let now = Instant::now();
        let (mut controller, keyboard) = connected(now);
        keyboard.deliver(vec![0xFE]);
        keyboard.deliver(vec![0x90, 0x3C, 0x40]);
        controller.tick(now);
        controller.tick(now + REPLY_TIMEOUT);
        assert_eq!(controller.error(), Some(OTHER_TRAFFIC));
    }

    #[test]
    fn unreadable_replies_are_shown_for_diagnosis() {
        let now = Instant::now();
        let (mut controller, keyboard) = connected(now);
        let mut odd = vec![0xF0, 0x43, 0x00, 0x7F, 0x1C, 0x7F, 0x7F, 0x07];
        odd.extend_from_slice(&[0x01; 30]);
        odd.push(0xF7);
        keyboard.deliver(odd);
        keyboard.deliver(vec![0xF0, 0x7E, 0x7F, 0xF7]);
        keyboard.deliver(vec![0xF8]);
        controller.tick(now);
        controller.tick(now + REPLY_TIMEOUT);
        assert_eq!(
            controller.error(),
            Some(
                "The keyboard answered, but SynthPilot couldn't understand the reply. Check that \
                 the selected synthesizer matches your keyboard. If it keeps happening, report it \
                 with these bytes (2 messages, first one 39 bytes): F0 43 00 7F 1C 7F 7F 07 01 01 \
                 01 01 01 01 01 01 01 01 01 01 01 01 01 01 …"
            )
        );
    }

    #[test]
    fn edits_on_the_keyboard_reach_the_editor() {
        let now = Instant::now();
        let (mut controller, keyboard) = connected(now);
        piano(&keyboard);
        settle(&mut controller, now);

        let device = modx_plus::ModxPlus::new();
        let engine = device.engine(awm2::ENGINE_ID).unwrap();
        let change = engine
            .encode(&ParameterChange::new(part::VOLUME, Location::part(1), 12))
            .unwrap();
        keyboard.deliver(change.into_bytes());
        keyboard.deliver(vec![0x90, 0x3C, 0x40]);
        controller.tick(now + QUIET_PERIOD);

        let part = controller
            .workspace()
            .unwrap()
            .sound()
            .unwrap()
            .part(1)
            .unwrap();
        assert_eq!(part.value(part::VOLUME, None), Some(12));
    }

    #[test]
    fn edits_are_forwarded_once_live() {
        let now = Instant::now();
        let (mut controller, keyboard) = connected(now);
        piano(&keyboard);
        settle(&mut controller, now);
        controller.dispatch(
            vec![Action::SetValue(awm2::ELEMENT_LEVEL, 100)],
            Vec::new(),
            now,
        );
        assert_eq!(
            sent(&keyboard).last().map(String::as_str),
            Some("F0 43 10 7F 1C 07 41 00 28 64 F7")
        );
        assert_eq!(controller.midi().sent(), 18);
    }

    #[test]
    fn reading_again_on_request() {
        let now = Instant::now();
        let (mut controller, keyboard) = connected(now);
        controller.tick(now + REPLY_TIMEOUT);
        controller.dispatch(vec![Action::ReadFromKeyboard], Vec::new(), now);
        assert_eq!(sent(&keyboard).len(), 34);
        assert!(controller.error().is_none());
    }

    #[test]
    fn disconnecting_returns_to_the_connection_screen() {
        let now = Instant::now();
        let (mut controller, keyboard) = connected(now);
        controller.dispatch(Vec::new(), vec![MidiAction::Disconnect], now);
        assert!(controller.workspace().is_none());
        assert!(controller.reading().is_none());
        assert!(controller.status().is_none());
        controller.dispatch(vec![Action::ReadFromKeyboard], Vec::new(), now);
        assert_eq!(sent(&keyboard).len(), 17);
    }

    #[test]
    fn nothing_opens_without_a_connection() {
        let mut controller = Controller::new(MidiLink::idle());
        controller.dispatch(
            vec![
                Action::SelectDevice(modx_plus::DEVICE_ID),
                Action::SelectPart(2),
            ],
            Vec::new(),
            Instant::now(),
        );
        controller.tick(Instant::now());
        assert!(controller.workspace().is_none());
        assert!(controller.reading().is_none());
    }

    #[test]
    fn a_failed_send_returns_to_the_connection_screen() {
        let now = Instant::now();
        let mut controller = Controller::new(MidiLink::idle());
        controller.connect(Box::new(Broken(Port {
            id: "MODX Port 1".to_owned(),
            name: "MODX Port 1".to_owned(),
        })));
        controller.dispatch(
            vec![Action::SelectDevice(modx_plus::DEVICE_ID)],
            Vec::new(),
            now,
        );
        assert!(controller.workspace().is_none());
        assert!(controller.midi().error().is_some());
    }

    struct Broken(Port);

    impl Connection for Broken {
        fn port(&self) -> &Port {
            &self.0
        }

        fn can_receive(&self) -> bool {
            true
        }

        fn send(&mut self, _message: &SysExMessage) -> Result<(), MidiError> {
            Err(MidiError::Send {
                port: self.0.name.clone(),
                reason: "unplugged".to_owned(),
            })
        }

        fn receive(&mut self) -> Vec<synthpilot_midi::Packet> {
            Vec::new()
        }
    }

    struct SendOnly(Port);

    impl Connection for SendOnly {
        fn port(&self) -> &Port {
            &self.0
        }

        fn can_receive(&self) -> bool {
            false
        }

        fn send(&mut self, _message: &SysExMessage) -> Result<(), MidiError> {
            Ok(())
        }

        fn receive(&mut self) -> Vec<synthpilot_midi::Packet> {
            Vec::new()
        }
    }

    #[test]
    fn send_only_connections_explain_the_limitation() {
        let mut controller = Controller::new(MidiLink::idle());
        controller.connect(Box::new(SendOnly(Port {
            id: "out".to_owned(),
            name: "Out".to_owned(),
        })));
        controller.dispatch(
            vec![Action::SelectDevice(modx_plus::DEVICE_ID)],
            Vec::new(),
            Instant::now(),
        );
        assert_eq!(controller.error(), Some(NO_INPUT));
        assert!(controller.reading().is_none());
    }
}
