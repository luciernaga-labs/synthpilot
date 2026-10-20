use std::time::{Duration, Instant};

use synthpilot_core::protocol::SysExMessage;
use synthpilot_midi::{Connection, MidiConnection, MidiError, Packet, Port};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MidiAction {
    Select(Port),
    Connect,
    Disconnect,
}

const SCAN_INTERVAL: Duration = Duration::from_secs(2);

pub(crate) struct MidiLink {
    ports: Vec<Port>,
    selected: Option<Port>,
    output: Option<Box<dyn Connection>>,
    sent: usize,
    error: Option<String>,
    scanning: bool,
    last_scan: Option<Instant>,
}

impl MidiLink {
    pub(crate) fn new() -> Self {
        let mut link = Self::idle();
        link.scanning = true;
        link.refresh();
        link.last_scan = Some(Instant::now());
        link
    }

    pub(crate) const fn idle() -> Self {
        Self {
            ports: Vec::new(),
            selected: None,
            output: None,
            sent: 0,
            error: None,
            scanning: false,
            last_scan: None,
        }
    }

    pub(crate) fn tick(&mut self, now: Instant) {
        let due = self
            .last_scan
            .is_none_or(|last| now.duration_since(last) >= SCAN_INTERVAL);
        if self.scanning && self.output.is_none() && due {
            self.refresh();
            self.last_scan = Some(now);
        }
    }

    pub(crate) fn ports(&self) -> &[Port] {
        &self.ports
    }

    pub(crate) fn selected(&self) -> Option<&Port> {
        self.selected.as_ref()
    }

    pub(crate) fn live_port(&self) -> Option<&Port> {
        self.output.as_deref().map(Connection::port)
    }

    pub(crate) fn can_receive(&self) -> bool {
        self.output.as_deref().is_some_and(Connection::can_receive)
    }

    pub(crate) fn receive(&mut self) -> Vec<Packet> {
        self.output
            .as_mut()
            .map(|output| output.receive())
            .unwrap_or_default()
    }

    pub(crate) const fn sent(&self) -> usize {
        self.sent
    }

    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub(crate) fn apply(&mut self, action: MidiAction) {
        self.error = None;
        match action {
            MidiAction::Select(port) => self.selected = Some(port),
            MidiAction::Connect => self.connect(),
            MidiAction::Disconnect => self.output = None,
        }
    }

    pub(crate) fn attach(&mut self, output: Box<dyn Connection>) {
        self.selected = Some(output.port().clone());
        self.output = Some(output);
        self.sent = 0;
    }

    pub(crate) fn send(&mut self, messages: &[SysExMessage]) {
        let Some(output) = self.output.as_mut() else {
            return;
        };
        for message in messages {
            if let Err(error) = output.send(message) {
                self.fail(&error);
                return;
            }
            self.sent += 1;
        }
    }

    fn refresh(&mut self) {
        match synthpilot_midi::output_ports() {
            Ok(ports) => {
                self.selected = preferred_port(self.selected.as_ref(), &ports);
                self.ports = ports;
                self.error = None;
            }
            Err(error) => self.fail(&error),
        }
    }

    fn connect(&mut self) {
        let Some(port) = self.selected.clone() else {
            self.error = Some("Choose your keyboard's MIDI port first.".to_owned());
            return;
        };
        match MidiConnection::open(&port) {
            Ok(connection) => self.attach(Box::new(connection)),
            Err(error) => self.fail(&error),
        }
    }

    fn fail(&mut self, error: &MidiError) {
        self.output = None;
        self.error = Some(error.to_string());
    }
}

fn preferred_port(current: Option<&Port>, ports: &[Port]) -> Option<Port> {
    current
        .filter(|current| ports.contains(current))
        .or_else(|| ports.iter().find(|port| port.name.contains("MODX")))
        .or_else(|| ports.first())
        .cloned()
}

#[cfg(test)]
mod tests {
    use synthpilot_midi::RecordingConnection;

    use super::*;

    fn message() -> SysExMessage {
        SysExMessage::new(vec![0xF0, 0x43, 0xF7]).unwrap()
    }

    #[test]
    fn idle_link_sends_nothing() {
        let mut link = MidiLink::idle();
        link.send(&[message()]);
        assert_eq!(link.sent(), 0);
        assert!(link.live_port().is_none());
    }

    #[test]
    fn live_link_forwards_messages() {
        let recording = RecordingConnection::new("MODX+");
        let mut link = MidiLink::idle();
        link.attach(Box::new(recording.clone()));
        link.send(&[message(), message()]);
        assert_eq!(link.sent(), 2);
        assert_eq!(recording.sent().len(), 2);
        assert_eq!(
            link.live_port().map(|port| port.name.as_str()),
            Some("MODX+")
        );

        link.apply(MidiAction::Disconnect);
        link.send(&[message()]);
        assert_eq!(recording.sent().len(), 2);
    }

    struct FailingOutput(Port);

    impl Connection for FailingOutput {
        fn port(&self) -> &Port {
            &self.0
        }

        fn can_receive(&self) -> bool {
            false
        }

        fn receive(&mut self) -> Vec<Packet> {
            Vec::new()
        }

        fn send(&mut self, _message: &SysExMessage) -> Result<(), MidiError> {
            Err(MidiError::Send {
                port: self.0.name.clone(),
                reason: "cable unplugged".to_owned(),
            })
        }
    }

    fn port(name: &str) -> Port {
        Port {
            id: name.to_owned(),
            name: name.to_owned(),
        }
    }

    #[test]
    fn idle_links_do_not_scan() {
        let mut link = MidiLink::idle();
        link.tick(Instant::now());
        assert_eq!(link.ports(), []);
        assert!(link.last_scan.is_none());
    }

    #[test]
    fn scanning_links_rescan_while_disconnected() {
        let mut link = MidiLink::idle();
        link.scanning = true;
        let now = Instant::now();
        link.tick(now);
        assert_eq!(link.last_scan, Some(now));
        link.tick(now + Duration::from_millis(500));
        assert_eq!(link.last_scan, Some(now));
        link.tick(now + SCAN_INTERVAL);
        assert_eq!(link.last_scan, Some(now + SCAN_INTERVAL));

        link.attach(Box::new(RecordingConnection::new("MODX Port 1")));
        link.tick(now + SCAN_INTERVAL * 2);
        assert_eq!(link.last_scan, Some(now + SCAN_INTERVAL));
    }

    #[test]
    fn startup_never_opens_a_connection() {
        let link = MidiLink::new();
        assert!(link.live_port().is_none());
        assert_eq!(link.sent(), 0);
    }

    #[test]
    fn send_failures_disconnect() {
        let mut link = MidiLink::idle();
        link.attach(Box::new(FailingOutput(port("MODX+"))));
        assert!(!link.can_receive());
        assert_eq!(link.receive(), [] as [synthpilot_midi::Packet; 0]);
        link.send(&[message(), message()]);
        assert!(link.live_port().is_none());
        assert_eq!(link.sent(), 0);
        assert_eq!(
            link.error(),
            Some(
                "Lost the connection to MODX+. Check the USB cable and that the keyboard is on, \
                 then connect again. Changes you already made are still on the keyboard. \
                 Details: cable unplugged"
            )
        );
    }

    #[test]
    fn connecting_without_a_port_explains_why() {
        let mut link = MidiLink::idle();
        link.apply(MidiAction::Connect);
        assert_eq!(
            link.error(),
            Some("Choose your keyboard's MIDI port first.")
        );
        assert!(link.live_port().is_none());
    }

    #[test]
    fn connecting_to_a_missing_port_stays_disconnected() {
        let mut link = MidiLink::idle();
        link.apply(MidiAction::Select(port("Not a real port")));
        assert_eq!(link.selected(), Some(&port("Not a real port")));
        link.apply(MidiAction::Connect);
        assert!(link.live_port().is_none());
        assert!(link.error().is_some());
    }

    #[test]
    fn refreshing_keeps_the_link_disconnected() {
        let mut link = MidiLink::idle();
        link.refresh();
        assert!(link.live_port().is_none());
        assert!(link.ports().iter().all(|port| !port.name.is_empty()));
    }

    #[test]
    fn preferred_port_keeps_the_current_choice() {
        let ports = [port("IAC Bus"), port("MODX+ Port1")];
        assert_eq!(
            preferred_port(Some(&port("IAC Bus")), &ports),
            Some(port("IAC Bus"))
        );
    }

    #[test]
    fn preferred_port_prefers_a_modx() {
        let ports = [port("IAC Bus"), port("MODX+ Port1")];
        assert_eq!(preferred_port(None, &ports), Some(port("MODX+ Port1")));
        assert_eq!(
            preferred_port(Some(&port("Unplugged")), &ports),
            Some(port("MODX+ Port1"))
        );
    }

    #[test]
    fn preferred_port_falls_back_to_the_first_one() {
        assert_eq!(
            preferred_port(None, &[port("IAC Bus"), port("Other")]),
            Some(port("IAC Bus"))
        );
        assert_eq!(preferred_port(None, &[]), None);
    }
}
