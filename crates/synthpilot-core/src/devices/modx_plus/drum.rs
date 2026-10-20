use super::PART_COUNT;
use super::catalog::{self, Entry};
use super::part;
use crate::domain::{
    EncodeError, EngineId, EngineProfile, Layout, ParameterChange, ParameterDefinition,
    ParameterId, Scope, ValueRange,
};
use crate::protocol::SysExMessage;
use crate::protocol::yamaha::DeviceNumber;

pub const ENGINE_ID: EngineId = EngineId::new("drum");
pub const KEY_COUNT: u8 = 73;
pub const KEY_LEVEL: ParameterId = ParameterId::new("drum.key-level");

pub(super) const FIRST_HIGH: u8 = 0x50;
pub(super) const LAST_HIGH: u8 = 0x5F;

pub(super) fn entries() -> Vec<Entry> {
    let mut entries = part::entries(ENGINE_ID, true);
    entries.push(Entry::unverified(
        ParameterDefinition::new(
            KEY_LEVEL,
            "Drum Key Level",
            ENGINE_ID,
            "Drum Keys",
            Scope::Element,
            ValueRange::new(0, 127),
        ),
        "SynthPilot can't edit individual drum keys yet. Use the Common settings above, or edit keys on the keyboard.",
    ));
    entries
}

pub struct Drum {
    device_number: DeviceNumber,
    entries: Vec<Entry>,
    parameters: Vec<ParameterDefinition>,
}

impl Drum {
    pub fn new(device_number: DeviceNumber) -> Self {
        let entries = entries();
        Self {
            device_number,
            parameters: catalog::definitions(&entries),
            entries,
        }
    }
}

impl EngineProfile for Drum {
    fn id(&self) -> EngineId {
        ENGINE_ID
    }

    fn name(&self) -> &'static str {
        "Drum"
    }

    fn description(&self) -> &'static str {
        "Drum Part with a different drum sound on each key. For now you can edit the Common settings, not individual keys."
    }

    fn element_label(&self) -> &'static str {
        "Key"
    }

    fn layout(&self) -> Layout {
        Layout {
            parts: PART_COUNT,
            elements: KEY_COUNT,
        }
    }

    fn parameters(&self) -> &[ParameterDefinition] {
        &self.parameters
    }

    fn encode(&self, change: &ParameterChange) -> Result<SysExMessage, EncodeError> {
        catalog::encode(&self.entries, self.layout(), self.device_number, change)
    }
}
