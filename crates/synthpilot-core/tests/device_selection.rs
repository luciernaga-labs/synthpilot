use synthpilot_core::devices::{self, modx_plus};
use synthpilot_core::domain::{DeviceId, EngineProfile, EngineSupport};

#[test]
fn catalog_lists_modx_plus() {
    let names: Vec<_> = devices::catalog()
        .iter()
        .map(devices::DeviceDescriptor::display_name)
        .collect();
    assert_eq!(names, ["Yamaha MODX+"]);
}

#[test]
fn selecting_a_device_loads_its_profile_without_hardware() {
    let descriptor = devices::catalog()
        .iter()
        .find(|descriptor| descriptor.id == modx_plus::DEVICE_ID)
        .expect("MODX+ is in the catalog");
    let profile = descriptor.load();
    assert_eq!(profile.id(), modx_plus::DEVICE_ID);
    assert_eq!(profile.display_name(), "Yamaha MODX+");
    assert_eq!(profile.part_count(), 16);
}

#[test]
fn loading_by_id_matches_the_catalog_entry() {
    let profile = devices::load(modx_plus::DEVICE_ID).expect("MODX+ loads");
    assert_eq!(profile.model(), "MODX+");
    assert!(devices::load(DeviceId::new("unknown-device")).is_none());
}

#[test]
fn modx_plus_declares_awm2_fm_x_and_drum() {
    let profile = devices::load(modx_plus::DEVICE_ID).unwrap();
    let engines: Vec<_> = profile
        .engines()
        .iter()
        .map(|engine| {
            (
                engine.name(),
                engine.element_label(),
                engine.layout().elements,
            )
        })
        .collect();
    assert_eq!(
        engines,
        [
            ("AWM2", "Element", 8),
            ("FM-X", "Operator", 8),
            ("Drum", "Key", 73)
        ]
    );
    for engine in profile.engines() {
        assert_ne!(engine.description(), "");
        assert_eq!(engine.support(), EngineSupport::Partial);
        assert_eq!(engine.layout().parts, 16);
    }
}

#[test]
fn engines_are_found_by_id() {
    let profile = devices::load(modx_plus::DEVICE_ID).unwrap();
    assert_eq!(
        profile
            .engine(modx_plus::awm2::ENGINE_ID)
            .map(EngineProfile::name),
        Some("AWM2")
    );
    assert_eq!(
        profile
            .engine(modx_plus::fm_x::ENGINE_ID)
            .map(EngineProfile::name),
        Some("FM-X")
    );
}
