pub mod modx_plus;

use std::fmt;

use crate::domain::{DeviceId, DeviceProfile};

#[derive(Clone, Copy)]
pub struct DeviceDescriptor {
    pub id: DeviceId,
    pub manufacturer: &'static str,
    pub model: &'static str,
    factory: fn() -> Box<dyn DeviceProfile>,
}

impl DeviceDescriptor {
    pub const fn new(
        id: DeviceId,
        manufacturer: &'static str,
        model: &'static str,
        factory: fn() -> Box<dyn DeviceProfile>,
    ) -> Self {
        Self {
            id,
            manufacturer,
            model,
            factory,
        }
    }

    pub fn load(&self) -> Box<dyn DeviceProfile> {
        (self.factory)()
    }

    pub fn display_name(&self) -> String {
        format!("{} {}", self.manufacturer, self.model)
    }
}

impl fmt::Debug for DeviceDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceDescriptor")
            .field("id", &self.id)
            .field("manufacturer", &self.manufacturer)
            .field("model", &self.model)
            .finish_non_exhaustive()
    }
}

const CATALOG: &[DeviceDescriptor] = &[modx_plus::DESCRIPTOR];

pub fn catalog() -> &'static [DeviceDescriptor] {
    CATALOG
}

pub fn load(id: DeviceId) -> Option<Box<dyn DeviceProfile>> {
    CATALOG
        .iter()
        .find(|descriptor| descriptor.id == id)
        .map(DeviceDescriptor::load)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptors_load_and_describe_their_device() {
        let descriptor = DeviceDescriptor::new(modx_plus::DEVICE_ID, "Yamaha", "MODX+", || {
            Box::new(modx_plus::ModxPlus::default())
        });
        assert_eq!(descriptor.display_name(), "Yamaha MODX+");
        assert_eq!(descriptor.load().id(), modx_plus::DEVICE_ID);
        let debug = format!("{descriptor:?}");
        assert!(debug.contains("yamaha-modx-plus") && debug.contains("MODX+"));
    }
}
