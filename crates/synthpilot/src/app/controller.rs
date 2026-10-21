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
/// Time the keyboard needs to load a sound into a Part before it is read back.
const LOAD_SETTLE: Duration = Duration::from_millis(900);
const NOT_LOADED: &str = "The keyboard didn't change the Part. On the MODX+, go to Utility > \
                          Settings > MIDI I/O and set MIDI I/O Mode to Multi, and check that \
                          Bank Select and Program Change are on in Utility > Settings > MIDI. \
                          Then try again.";

/// A sound sent to a Part, waiting to be read back.
struct Loading {
    part: u8,
    name: &'static str,
    before: Option<(String, Option<String>)>,
    read_at: Instant,
    read_started: bool,
}

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
    loading: Option<Loading>,
    status: Option<String>,
    error: Option<String>,
}

impl Controller {
    pub(crate) const fn new(midi: MidiLink) -> Self {
        Self {
            workspace: None,
            midi,
            reading: None,
            loading: None,
            status: None,
            error: None,
        }
    }

    /// The Part a sound is being loaded into, if any.
    pub(crate) fn loading(&self) -> Option<u8> {
        self.loading.as_ref().map(|loading| loading.part)
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
        let mut reshaped = false;
        for action in actions {
            if action == Action::ReadFromKeyboard {
                self.start_reading(now);
                continue;
            }
            if let Action::LoadPreset { part, number } = action {
                self.load_preset(part, number, now);
                continue;
            }
            match (&mut self.workspace, action) {
                (None, Action::SelectDevice(id)) if self.midi.live_port().is_some() => {
                    self.workspace = devices::load(id).map(Workspace::new);
                    self.start_reading(now);
                }
                (Some(workspace), action) => {
                    let messages = workspace.apply(action);
                    reshaped |= !messages.is_empty() && workspace.reshapes(action);
                    self.midi.send(&messages);
                }
                (None, _) => {}
            }
        }
        if reshaped && self.reading.is_none() {
            self.start_reading(now);
        }
        self.leave_if_disconnected();
    }

    fn load_preset(&mut self, part: u8, number: i32, now: Instant) {
        let Some(workspace) = self.workspace.as_mut() else {
            return;
        };
        let device = workspace.device();
        let name = usize::try_from(number - 1)
            .ok()
            .and_then(|index| device.presets().get(index))
            .map(|preset| preset.name);
        let (Some(messages), Some(name)) = (device.load_preset(part, number), name) else {
            self.error = Some("This sound can't be loaded into that Part.".to_owned());
            return;
        };
        let before = workspace
            .sound()
            .and_then(|sound| sound.part(part))
            .map(|values| {
                (
                    values.engine().as_str().to_owned(),
                    values.name().map(ToOwned::to_owned),
                )
            });
        if !self.midi.send_channel(&messages) {
            return;
        }
        workspace.apply(Action::SelectPart(part));
        self.error = None;
        self.status = Some(format!("Loading {name} into Part {part}…"));
        self.loading = Some(Loading {
            part,
            name,
            before,
            read_at: now + LOAD_SETTLE,
            read_started: false,
        });
    }

    /// After a sound was read back, says whether the Part took it.
    fn confirm_loading(&mut self, loading: &Loading) {
        let Some(workspace) = self.workspace.as_ref() else {
            return;
        };
        let after = workspace
            .sound()
            .and_then(|sound| sound.part(loading.part))
            .map(|values| {
                (
                    values.engine().as_str().to_owned(),
                    values.name().map(ToOwned::to_owned),
                )
            });
        let part = loading.part;
        match after {
            Some((_, Some(name))) if name.eq_ignore_ascii_case(loading.name) => {
                self.status = Some(format!("Loaded {name} into Part {part}."));
            }
            Some(after) if Some(&after) != loading.before.as_ref() => {
                let shown = after.1.as_deref().unwrap_or("an unnamed sound");
                self.status = Some(format!(
                    "Loaded {} into Part {part}. The keyboard calls this Part {shown}: for a Performance with several Parts it loads only the first one.",
                    loading.name
                ));
            }
            _ => {
                self.status = None;
                self.error = Some(NOT_LOADED.to_owned());
            }
        }
    }

    fn leave_if_disconnected(&mut self) {
        if self.midi.live_port().is_none() {
            self.workspace = None;
            self.reading = None;
            self.loading = None;
            self.status = None;
        }
    }

    pub(crate) fn tick(&mut self, now: Instant) {
        self.midi.tick(now);
        self.leave_if_disconnected();
        let due = self
            .loading
            .as_ref()
            .is_some_and(|loading| !loading.read_started && now >= loading.read_at);
        if due && self.reading.is_none() {
            self.start_reading(now);
            if self.reading.is_some() {
                if let Some(loading) = self.loading.as_mut() {
                    loading.read_started = true;
                }
            } else {
                self.loading = None;
            }
        }
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
                None => match received {
                    Received::Part(data) => workspace.receive(&data),
                    Received::Common(values) => workspace.receive_common(&values),
                    _ => {}
                },
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
            if let Some(loading) = self.loading.take_if(|loading| loading.read_started) {
                self.confirm_loading(&loading);
            }
        } else if timed_out {
            self.error = self.reading.take().map(|reading| reading.unreadable());
            self.status = None;
            self.loading = None;
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
    use synthpilot_core::protocol::yamaha::{self, Address, DeviceNumber, ModelId};
    use synthpilot_core::protocol::{ChannelMessage, SysExMessage};
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

    fn part_named(keyboard: &RecordingConnection, part: u8, name: &str) {
        let mut part_block = vec![0x40; 0x79];
        part_block[..20].copy_from_slice(format!("{name:<20}").as_bytes());
        keyboard.deliver(dump(0x0E, 0x52, part - 1, &[]));
        keyboard.deliver(dump(0x31, part - 1, 0x00, &part_block));
        keyboard.deliver(dump(0x41, part - 1, 0x00, &[0x40; 0x67]));
        keyboard.deliver(dump(0x0F, 0x52, part - 1, &[]));
    }

    fn loaded(name: &str) -> (Controller, RecordingConnection, Instant) {
        let now = Instant::now();
        let (mut controller, keyboard) = connected(now);
        piano(&keyboard);
        settle(&mut controller, now);
        let before = keyboard.sent().len();
        controller.dispatch(
            vec![Action::LoadPreset {
                part: 7,
                number: 955,
            }],
            Vec::new(),
            now,
        );
        let channel: Vec<String> = keyboard
            .sent_channel()
            .iter()
            .map(ChannelMessage::to_hex)
            .collect();
        assert_eq!(channel, ["B6 00 3F", "B6 20 07", "C6 3A"]);
        assert_eq!(controller.status(), Some("Loading Gamelan into Part 7…"));
        assert_eq!(controller.loading(), Some(7));
        assert_eq!(controller.workspace().unwrap().part(), 7);
        controller.tick(now + LOAD_SETTLE / 2);
        assert_eq!(keyboard.sent().len(), before, "waits for the keyboard");
        let later = now + LOAD_SETTLE;
        controller.tick(later);
        assert!(controller.reading().is_some());
        assert!(keyboard.sent().len() > before);
        piano(&keyboard);
        if !name.is_empty() {
            part_named(&keyboard, 7, name);
        }
        settle(&mut controller, later);
        assert_eq!(controller.loading(), None);
        (controller, keyboard, later)
    }

    #[test]
    fn presets_load_into_a_part_and_are_read_back() {
        let (controller, _, _) = loaded("Gamelan");
        assert_eq!(controller.status(), Some("Loaded Gamelan into Part 7."));
        assert_eq!(controller.error(), None);
        let part = controller.workspace().unwrap().sound().unwrap().part(7);
        assert_eq!(part.and_then(|part| part.name()), Some("Gamelan"));
    }

    #[test]
    fn a_multi_part_preset_reports_the_part_it_loaded() {
        let (controller, _, _) = loaded("Bali Bells");
        assert!(
            controller
                .status()
                .unwrap()
                .starts_with("Loaded Gamelan into Part 7. The keyboard calls this Part Bali Bells")
        );
    }

    #[test]
    fn an_unchanged_part_explains_the_keyboard_settings() {
        let (controller, _, _) = loaded("");
        assert_eq!(controller.status(), None);
        assert!(
            controller
                .error()
                .unwrap()
                .contains("MIDI I/O Mode to Multi")
        );
    }

    #[test]
    fn unknown_presets_are_refused() {
        let now = Instant::now();
        let (mut controller, keyboard) = connected(now);
        piano(&keyboard);
        settle(&mut controller, now);
        controller.dispatch(
            vec![Action::LoadPreset { part: 3, number: 0 }],
            Vec::new(),
            now,
        );
        assert!(keyboard.sent_channel().is_empty());
        assert_eq!(
            controller.error(),
            Some("This sound can't be loaded into that Part.")
        );
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
    fn effect_edits_on_the_keyboard_reach_the_editor() {
        let now = Instant::now();
        let (mut controller, keyboard) = connected(now);
        piano(&keyboard);
        settle(&mut controller, now);

        let device = modx_plus::ModxPlus::new();
        let effects = device.common().unwrap();
        let change = effects
            .encode(&ParameterChange::new(
                modx_plus::effects::VARIATION_RETURN,
                Location::performance(),
                33,
            ))
            .unwrap();
        keyboard.deliver(change.into_bytes());
        controller.tick(now + QUIET_PERIOD);

        let common = controller
            .workspace()
            .unwrap()
            .sound()
            .unwrap()
            .common()
            .unwrap();
        assert_eq!(
            common.value(modx_plus::effects::VARIATION_RETURN, None),
            Some(33)
        );
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
    fn changing_an_effect_type_reads_the_sound_again() {
        let now = Instant::now();
        let (mut controller, keyboard) = connected(now);
        piano(&keyboard);
        settle(&mut controller, now);
        controller.dispatch(
            vec![Action::SetValue(part::INSERTION_A_TYPE, 15)],
            Vec::new(),
            now,
        );
        let sent = sent(&keyboard);
        assert_eq!(sent[17], "F0 43 10 7F 1C 07 31 20 00 03 00 F7");
        assert_eq!(sent.len(), 35);
        assert_eq!(sent[18], "F0 43 20 7F 1C 07 0E 52 00 F7");
        assert!(controller.reading().is_some());
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
