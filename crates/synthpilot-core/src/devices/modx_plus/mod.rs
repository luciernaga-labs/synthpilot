pub mod awm2;
mod catalog;
pub mod drum;
pub mod fm_x;
pub mod part;
mod reader;

pub use awm2::Awm2;
pub use drum::Drum;
pub use fm_x::FmX;

use reader::Decoder;

use super::DeviceDescriptor;
use crate::domain::{DeviceId, DeviceProfile, EngineProfile, Received};
use crate::protocol::SysExMessage;
use crate::protocol::yamaha::{DeviceNumber, ModelId};

pub const DEVICE_ID: DeviceId = DeviceId::new("yamaha-modx-plus");
pub const PART_COUNT: u8 = 16;
pub const DATA_LIST: &str = "Yamaha MODX+ Data List (modx_plus_en_dl_a0.pdf)";
pub const DESCRIPTOR: DeviceDescriptor =
    DeviceDescriptor::new(DEVICE_ID, "Yamaha", "MODX+", create);

const MODEL_ID: ModelId = ModelId::new(&[0x7F, 0x1C, 0x07]);

fn create() -> Box<dyn DeviceProfile> {
    Box::new(ModxPlus::new())
}

pub struct ModxPlus {
    device_number: DeviceNumber,
    engines: Vec<Box<dyn EngineProfile>>,
    decoder: Decoder,
}

impl ModxPlus {
    pub fn new() -> Self {
        Self::with_device_number(DeviceNumber::default())
    }

    pub fn with_device_number(device_number: DeviceNumber) -> Self {
        Self {
            device_number,
            engines: vec![
                Box::new(Awm2::new(device_number)),
                Box::new(FmX::new(device_number)),
                Box::new(Drum::new(device_number)),
            ],
            decoder: Decoder::new(),
        }
    }
}

impl Default for ModxPlus {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceProfile for ModxPlus {
    fn id(&self) -> DeviceId {
        DEVICE_ID
    }

    fn manufacturer(&self) -> &'static str {
        "Yamaha"
    }

    fn model(&self) -> &'static str {
        "MODX+"
    }

    fn part_count(&self) -> u8 {
        PART_COUNT
    }

    fn engines(&self) -> &[Box<dyn EngineProfile>] {
        &self.engines
    }

    fn read_requests(&self) -> Vec<SysExMessage> {
        Decoder::requests(self.device_number)
    }

    fn decode(&self, bytes: &[u8]) -> Received {
        self.decoder.decode(bytes)
    }
}
