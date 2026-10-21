use super::{
    DeviceId, EncodeError, EngineId, Layout, ListedValue, Location, ParameterChange,
    ParameterDefinition, ParameterId, Part, Received,
};
use crate::protocol::{ChannelMessage, SysExMessage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EngineSupport {
    Complete,
    Partial,
    NotImplemented,
}

impl EngineSupport {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Complete => "Fully editable",
            Self::Partial => "Partly editable",
            Self::NotImplemented => "Not editable yet",
        }
    }
}

pub trait EngineProfile: Send + Sync {
    fn id(&self) -> EngineId;

    fn name(&self) -> &'static str;

    fn description(&self) -> &'static str;

    fn element_label(&self) -> &'static str;

    fn layout(&self) -> Layout;

    fn parameters(&self) -> &[ParameterDefinition];

    fn encode(&self, change: &ParameterChange) -> Result<SysExMessage, EncodeError>;

    fn parameter(&self, id: ParameterId) -> Option<&ParameterDefinition> {
        self.parameters()
            .iter()
            .find(|parameter| parameter.id == id)
    }

    fn default_value(&self, parameter: &ParameterDefinition, _location: Location) -> Option<i32> {
        parameter.default
    }

    fn refine(&self, parameter: &ParameterDefinition, _part: &Part) -> Option<ParameterDefinition> {
        Some(*parameter)
    }

    fn dependents(&self, _control: &ParameterDefinition, _value: i32) -> Vec<ParameterDefinition> {
        Vec::new()
    }

    fn reshapes(&self, _parameter: ParameterId) -> bool {
        false
    }

    fn support(&self) -> EngineSupport {
        let parameters = self.parameters();
        let implemented = parameters
            .iter()
            .filter(|parameter| parameter.is_implemented())
            .count();
        match implemented {
            0 => EngineSupport::NotImplemented,
            count if count == parameters.len() => EngineSupport::Complete,
            _ => EngineSupport::Partial,
        }
    }
}

pub trait DeviceProfile: Send + Sync {
    fn id(&self) -> DeviceId;

    fn manufacturer(&self) -> &'static str;

    fn model(&self) -> &'static str;

    fn part_count(&self) -> u8;

    fn engines(&self) -> &[Box<dyn EngineProfile>];

    fn common(&self) -> Option<&dyn EngineProfile> {
        None
    }

    fn engine(&self, id: EngineId) -> Option<&dyn EngineProfile> {
        self.engines()
            .iter()
            .find(|engine| engine.id() == id)
            .map(AsRef::as_ref)
    }

    fn display_name(&self) -> String {
        format!("{} {}", self.manufacturer(), self.model())
    }

    fn read_requests(&self) -> Vec<SysExMessage> {
        Vec::new()
    }

    fn decode(&self, _bytes: &[u8]) -> Received {
        Received::Ignored
    }

    /// The instrument's preset sounds, numbered from 1, that can be loaded
    /// into a Part with [`DeviceProfile::load_preset`].
    fn presets(&self) -> &'static [ListedValue] {
        &[]
    }

    /// Messages that load preset `number` (1-based, see
    /// [`DeviceProfile::presets`]) into Part `part`, replacing its sound.
    fn load_preset(&self, _part: u8, _number: i32) -> Option<Vec<ChannelMessage>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Scope, Support, ValueRange};
    use crate::protocol::SysExMessage;

    struct FakeEngine(Vec<ParameterDefinition>);

    impl EngineProfile for FakeEngine {
        fn id(&self) -> EngineId {
            EngineId::new("fake")
        }

        fn name(&self) -> &'static str {
            "Fake"
        }

        fn description(&self) -> &'static str {
            "A test engine."
        }

        fn element_label(&self) -> &'static str {
            "Element"
        }

        fn layout(&self) -> Layout {
            Layout {
                parts: 1,
                elements: 1,
            }
        }

        fn parameters(&self) -> &[ParameterDefinition] {
            &self.0
        }

        fn encode(&self, change: &ParameterChange) -> Result<SysExMessage, EncodeError> {
            Err(EncodeError::NotImplemented(change.parameter))
        }
    }

    fn parameter(id: &'static str, implemented: bool) -> ParameterDefinition {
        let definition = ParameterDefinition::new(
            ParameterId::new(id),
            id,
            EngineId::new("fake"),
            "Test",
            Scope::Part,
            ValueRange::new(0, 1),
        )
        .with_default(1);
        if implemented {
            definition.with_support(Support::Implemented(crate::domain::SourceReference {
                document: "Doc",
                page: 1,
                table: "Table",
            }))
        } else {
            definition
        }
    }

    #[test]
    fn support_reflects_implemented_parameters() {
        let none = FakeEngine(vec![parameter("a", false)]);
        let partial = FakeEngine(vec![parameter("a", true), parameter("b", false)]);
        let complete = FakeEngine(vec![parameter("a", true)]);
        assert_eq!(none.support(), EngineSupport::NotImplemented);
        assert_eq!(partial.support(), EngineSupport::Partial);
        assert_eq!(complete.support(), EngineSupport::Complete);
        assert_eq!(EngineSupport::Complete.label(), "Fully editable");
        assert_eq!(EngineSupport::Partial.label(), "Partly editable");
        assert_eq!(EngineSupport::NotImplemented.label(), "Not editable yet");
    }

    #[test]
    fn engines_have_static_parameters_by_default() {
        let engine = FakeEngine(vec![parameter("a", true)]);
        let definition = *engine.parameter(ParameterId::new("a")).unwrap();
        let device = DeviceForTests;
        let mut sound = crate::domain::Sound::new(&device);
        sound.load(1, &engine, None, &[]).unwrap();
        let part = sound.part(1).unwrap();
        assert_eq!(engine.refine(&definition, part), Some(definition));
        assert!(engine.dependents(&definition, 1).is_empty());
        assert!(!engine.reshapes(definition.id));
        assert!(device.common().is_none());
    }

    struct DeviceForTests;

    impl DeviceProfile for DeviceForTests {
        fn id(&self) -> DeviceId {
            DeviceId::new("test")
        }

        fn manufacturer(&self) -> &'static str {
            "Test"
        }

        fn model(&self) -> &'static str {
            "Device"
        }

        fn part_count(&self) -> u8 {
            1
        }

        fn engines(&self) -> &[Box<dyn EngineProfile>] {
            &[]
        }
    }

    #[test]
    fn default_values_come_from_the_definition() {
        let engine = FakeEngine(vec![parameter("a", true)]);
        let definition = engine.parameter(ParameterId::new("a")).unwrap();
        assert_eq!(engine.default_value(definition, Location::part(1)), Some(1));
        assert!(engine.parameter(ParameterId::new("missing")).is_none());
    }
}
