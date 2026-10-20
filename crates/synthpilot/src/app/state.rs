use synthpilot_core::devices;
use synthpilot_core::domain::{
    DeviceId, DeviceProfile, EncodeError, EngineProfile, Location, ParameterChange,
    ParameterDefinition, ParameterId, PartData, Scope, Sound,
};
use synthpilot_core::protocol::SysExMessage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    ReadFromKeyboard,
    SelectDevice(DeviceId),
    SelectPart(u8),
    SelectElement(u8),
    SelectParameter(ParameterId),
    SetValue(ParameterId, i32),
}

pub(crate) struct Preview {
    pub(crate) definition: ParameterDefinition,
    pub(crate) location: Location,
    pub(crate) result: Result<SysExMessage, EncodeError>,
}

pub(crate) struct Workspace {
    device: Box<dyn DeviceProfile>,
    sound: Option<Sound>,
    part: u8,
    element: u8,
    selected: Option<ParameterId>,
    error: Option<String>,
}

impl Workspace {
    pub(crate) fn new(device: Box<dyn DeviceProfile>) -> Self {
        Self {
            device,
            sound: None,
            part: 1,
            element: 1,
            selected: None,
            error: None,
        }
    }

    pub(crate) fn device(&self) -> &dyn DeviceProfile {
        self.device.as_ref()
    }

    pub(crate) fn sound(&self) -> Option<&Sound> {
        self.sound.as_ref()
    }

    pub(crate) const fn part(&self) -> u8 {
        self.part
    }

    pub(crate) const fn element(&self) -> u8 {
        self.element
    }

    pub(crate) const fn selected(&self) -> Option<ParameterId> {
        self.selected
    }

    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub(crate) fn engine(&self) -> Option<&dyn EngineProfile> {
        let part = self.sound.as_ref()?.part(self.part)?;
        self.device.engine(part.engine())
    }

    pub(crate) fn location(&self, definition: &ParameterDefinition) -> Location {
        Location::for_scope(definition.scope, self.part, self.element)
    }

    pub(crate) fn value(&self, definition: &ParameterDefinition) -> Option<i32> {
        let part = self.sound.as_ref()?.part(self.part)?;
        let element = match definition.scope {
            Scope::Part => None,
            Scope::Element => Some(self.element),
        };
        part.value(definition.id, element)
    }

    pub(crate) fn preview(&self) -> Option<Preview> {
        let engine = self.engine()?;
        let definition = *engine.parameter(self.selected?)?;
        let location = self.location(&definition);
        let value = self
            .value(&definition)
            .unwrap_or_else(|| definition.range.min());
        let result = engine.encode(&ParameterChange::new(definition.id, location, value));
        Some(Preview {
            definition,
            location,
            result,
        })
    }

    pub(crate) fn load(&mut self, sound: Sound) {
        self.part = (1..=sound.part_count())
            .find(|number| sound.part(*number).is_some())
            .unwrap_or(1);
        self.sound = Some(sound);
        self.element = 1;
        self.selected = self.first_parameter();
    }

    pub(crate) fn receive(&mut self, data: &PartData) {
        let Some(sound) = self.sound.as_mut() else {
            return;
        };
        let Some(engine) = sound
            .part(data.part)
            .and_then(|part| self.device.engine(part.engine()))
        else {
            return;
        };
        for change in &data.values {
            let _ = sound.store(engine, *change);
        }
    }

    pub(crate) fn apply(&mut self, action: Action) -> Vec<SysExMessage> {
        self.error = None;
        match action {
            Action::SetValue(id, value) => return self.set_value(id, value).into_iter().collect(),
            Action::SelectDevice(id) => self.select_device(id),
            Action::SelectPart(part) => {
                self.part = part;
                self.element = 1;
                self.selected = self.first_parameter();
            }
            Action::SelectElement(element) => self.element = element,
            Action::SelectParameter(id) => self.selected = Some(id),
            Action::ReadFromKeyboard => {}
        }
        Vec::new()
    }

    fn select_device(&mut self, id: DeviceId) {
        if self.device.id() == id {
            return;
        }
        match devices::load(id) {
            Some(device) => *self = Self::new(device),
            None => self.error = Some("This synthesizer isn't supported yet.".to_owned()),
        }
    }

    fn set_value(&mut self, id: ParameterId, value: i32) -> Option<SysExMessage> {
        self.selected = Some(id);
        let sound = self.sound.as_mut()?;
        let engine = sound
            .part(self.part)
            .and_then(|part| self.device.engine(part.engine()))?;
        let definition = engine.parameter(id)?;
        let location = Location::for_scope(definition.scope, self.part, self.element);
        match sound.apply(engine, ParameterChange::new(id, location, value)) {
            Ok(message) => Some(message),
            Err(error) => {
                self.error = Some(error.to_string());
                None
            }
        }
    }

    fn first_parameter(&self) -> Option<ParameterId> {
        self.engine()?
            .parameters()
            .iter()
            .find(|parameter| parameter.is_implemented())
            .map(|parameter| parameter.id)
    }
}

#[cfg(test)]
mod tests {
    use synthpilot_core::devices::modx_plus::{self, awm2, fm_x, part};
    use synthpilot_core::domain::EngineId;

    use super::*;

    fn workspace() -> Workspace {
        Workspace::new(devices::load(modx_plus::DEVICE_ID).unwrap())
    }

    fn with(engine: EngineId) -> Workspace {
        let mut workspace = workspace();
        let sound = {
            let device = workspace.device();
            Sound::with_engine(device, device.engine(engine).unwrap())
        };
        workspace.load(sound);
        workspace
    }

    fn definition(workspace: &Workspace, id: ParameterId) -> ParameterDefinition {
        *workspace.engine().unwrap().parameter(id).unwrap()
    }

    #[test]
    fn starts_without_a_sound() {
        let workspace = workspace();
        assert!(workspace.sound().is_none());
        assert!(workspace.engine().is_none());
        assert!(workspace.preview().is_none());
    }

    #[test]
    fn loading_selects_the_first_implemented_parameter() {
        let workspace = with(fm_x::ENGINE_ID);
        assert_eq!(workspace.engine().map(EngineProfile::name), Some("FM-X"));
        assert_eq!(workspace.selected(), Some(part::VOLUME));
        let preview = workspace.preview().unwrap();
        assert_eq!(
            preview.result.unwrap().to_hex(),
            "F0 43 10 7F 1C 07 31 00 24 64 F7"
        );
    }

    #[test]
    fn editing_updates_value_and_preview() {
        let mut workspace = with(awm2::ENGINE_ID);
        workspace.apply(Action::SelectElement(2));
        workspace.apply(Action::SetValue(awm2::FILTER_CUTOFF, 200));

        let cutoff = definition(&workspace, awm2::FILTER_CUTOFF);
        assert_eq!(workspace.value(&cutoff), Some(200));
        let preview = workspace.preview().unwrap();
        assert_eq!(preview.location, Location::element(1, 2));
        assert_eq!(
            preview.result.unwrap().to_hex(),
            "F0 43 10 7F 1C 07 42 10 01 01 48 F7"
        );
    }

    #[test]
    fn unimplemented_parameters_preview_as_errors() {
        let mut workspace = with(awm2::ENGINE_ID);
        workspace.apply(Action::SelectParameter(awm2::FILTER_TYPE));
        let preview = workspace.preview().unwrap();
        assert_eq!(
            preview.result,
            Err(EncodeError::NotImplemented(awm2::FILTER_TYPE))
        );
    }

    #[test]
    fn edits_return_the_message_to_send() {
        let mut workspace = with(awm2::ENGINE_ID);
        let sent = workspace.apply(Action::SetValue(awm2::ELEMENT_LEVEL, 100));
        let hex: Vec<_> = sent.iter().map(SysExMessage::to_hex).collect();
        assert_eq!(hex, ["F0 43 10 7F 1C 07 41 00 28 64 F7"]);

        assert!(
            workspace
                .apply(Action::SetValue(awm2::ELEMENT_LEVEL, 999))
                .is_empty()
        );
        assert!(workspace.error().is_some());
        assert!(workspace.apply(Action::SelectElement(2)).is_empty());
    }

    #[test]
    fn loading_a_sound_selects_the_first_used_part() {
        let mut workspace = workspace();
        let mut sound = Sound::new(workspace.device());
        let engine = workspace.device().engine(fm_x::ENGINE_ID).unwrap();
        sound
            .load(3, engine, Some("Bells".to_owned()), &[])
            .unwrap();
        workspace.load(sound);
        assert_eq!(workspace.part(), 3);
        assert_eq!(workspace.element(), 1);
        assert_eq!(workspace.selected(), Some(part::VOLUME));
        assert_eq!(
            workspace.sound().and_then(|sound| sound.part(3)?.name()),
            Some("Bells")
        );

        workspace.load(Sound::new(workspace.device()));
        assert_eq!(workspace.part(), 1);
        assert!(workspace.engine().is_none());
    }

    #[test]
    fn selecting_parts_resets_the_element() {
        let mut workspace = workspace();
        let mut sound = Sound::new(workspace.device());
        let device = workspace.device();
        sound
            .load(1, device.engine(awm2::ENGINE_ID).unwrap(), None, &[])
            .unwrap();
        sound
            .load(2, device.engine(fm_x::ENGINE_ID).unwrap(), None, &[])
            .unwrap();
        workspace.load(sound);
        workspace.apply(Action::SelectElement(4));
        workspace.apply(Action::SelectPart(2));
        assert_eq!(workspace.part(), 2);
        assert_eq!(workspace.element(), 1);
        assert_eq!(workspace.engine().map(EngineProfile::name), Some("FM-X"));
    }

    #[test]
    fn changes_from_the_keyboard_update_the_sound() {
        let mut workspace = with(awm2::ENGINE_ID);
        workspace.receive(&PartData {
            part: 1,
            engine: None,
            name: None,
            values: vec![
                ParameterChange::new(awm2::ELEMENT_LEVEL, Location::element(1, 2), 33),
                ParameterChange::new(fm_x::ALGORITHM, Location::part(1), 3),
            ],
        });
        workspace.apply(Action::SelectElement(2));
        let level = definition(&workspace, awm2::ELEMENT_LEVEL);
        assert_eq!(workspace.value(&level), Some(33));

        workspace.receive(&PartData {
            part: 5,
            values: vec![ParameterChange::new(part::VOLUME, Location::part(5), 1)],
            ..PartData::default()
        });
        assert!(workspace.sound().unwrap().part(5).is_none());
    }

    #[test]
    fn changes_without_a_sound_are_ignored() {
        let mut workspace = workspace();
        workspace.receive(&PartData {
            part: 1,
            values: vec![ParameterChange::new(part::VOLUME, Location::part(1), 1)],
            ..PartData::default()
        });
        assert!(workspace.sound().is_none());
        assert!(workspace.apply(Action::ReadFromKeyboard).is_empty());
        assert!(
            workspace
                .apply(Action::SetValue(awm2::ELEMENT_LEVEL, 1))
                .is_empty()
        );
    }

    #[test]
    fn selecting_the_same_device_keeps_the_sound() {
        let mut workspace = with(awm2::ENGINE_ID);
        workspace.apply(Action::SelectDevice(modx_plus::DEVICE_ID));
        assert!(workspace.sound().is_some());
        assert_eq!(workspace.device().id(), modx_plus::DEVICE_ID);
    }

    #[test]
    fn unknown_devices_are_reported() {
        let mut workspace = workspace();
        workspace.apply(Action::SelectDevice(DeviceId::new("unknown")));
        assert_eq!(
            workspace.error(),
            Some("This synthesizer isn't supported yet.")
        );
    }

    #[test]
    fn part_values_follow_the_selected_element() {
        let mut workspace = with(awm2::ENGINE_ID);
        let switch = definition(&workspace, awm2::ELEMENT_SWITCH);
        assert_eq!(workspace.value(&switch), Some(1));
        workspace.apply(Action::SelectElement(3));
        assert_eq!(workspace.element(), 3);
        assert_eq!(workspace.value(&switch), Some(0));
        assert_eq!(workspace.location(&switch), Location::element(1, 3));
    }
}
