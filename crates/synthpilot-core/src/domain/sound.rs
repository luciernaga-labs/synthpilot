use std::collections::BTreeMap;

use super::{
    DeviceId, DeviceProfile, EditError, EngineId, EngineProfile, Location, ParameterChange,
    ParameterId, Scope, ValidationError,
};
use crate::protocol::SysExMessage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    engine: EngineId,
    name: Option<String>,
    values: BTreeMap<(ParameterId, Option<u8>), i32>,
}

impl Part {
    fn initialized(number: u8, engine: &dyn EngineProfile) -> Self {
        let elements = engine.layout().elements;
        let mut values = BTreeMap::new();
        for parameter in engine.parameters() {
            let locations: Vec<Location> = match parameter.scope {
                Scope::Performance => vec![Location::performance()],
                Scope::Part => vec![Location::part(number)],
                Scope::Element => (1..=elements)
                    .map(|element| Location::element(number, element))
                    .collect(),
            };
            for location in locations {
                if let Some(value) = engine.default_value(parameter, location) {
                    values.insert((parameter.id, location.element_number()), value);
                }
            }
        }
        Self {
            engine: engine.id(),
            name: None,
            values,
        }
    }

    pub const fn engine(&self) -> EngineId {
        self.engine
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn value(&self, parameter: ParameterId, element: Option<u8>) -> Option<i32> {
        self.values.get(&(parameter, element)).copied()
    }

    #[must_use]
    pub fn with(&self, change: &ParameterChange) -> Self {
        let mut part = self.clone();
        part.values.insert(
            (change.parameter, change.location.element_number()),
            change.value,
        );
        part
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sound {
    device: DeviceId,
    name: Option<String>,
    common: Option<Part>,
    parts: Vec<Option<Part>>,
}

impl Sound {
    pub fn new(device: &dyn DeviceProfile) -> Self {
        Self {
            device: device.id(),
            name: None,
            common: device.common().map(|engine| Part::initialized(0, engine)),
            parts: vec![None; usize::from(device.part_count())],
        }
    }

    pub fn with_engine(device: &dyn DeviceProfile, engine: &dyn EngineProfile) -> Self {
        let mut sound = Self::new(device);
        sound.parts[0] = Some(Part::initialized(1, engine));
        sound
    }

    pub const fn device(&self) -> DeviceId {
        self.device
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn rename(&mut self, name: Option<String>) {
        self.name = name;
    }

    pub fn part_count(&self) -> u8 {
        u8::try_from(self.parts.len()).unwrap_or(u8::MAX)
    }

    pub fn common(&self) -> Option<&Part> {
        self.common.as_ref()
    }

    pub fn load_common(&mut self, engine: &dyn EngineProfile, values: &[ParameterChange]) -> usize {
        let mut common = Part::initialized(0, engine);
        let layout = engine.layout();
        let mut loaded = 0;
        for change in values {
            let valid = engine
                .parameter(change.parameter)
                .is_some_and(|definition| definition.validate(change, layout).is_ok());
            if valid {
                common.values.insert((change.parameter, None), change.value);
                loaded += 1;
            }
        }
        self.common = Some(common);
        loaded
    }

    pub fn part(&self, number: u8) -> Option<&Part> {
        self.slot(number).ok().and_then(Option::as_ref)
    }

    pub fn assign(
        &mut self,
        number: u8,
        engine: &dyn EngineProfile,
    ) -> Result<(), ValidationError> {
        self.slot(number)?;
        self.parts[usize::from(number - 1)] = Some(Part::initialized(number, engine));
        Ok(())
    }

    pub fn clear(&mut self, number: u8) -> Result<(), ValidationError> {
        self.slot(number)?;
        self.parts[usize::from(number - 1)] = None;
        Ok(())
    }

    pub fn load(
        &mut self,
        number: u8,
        engine: &dyn EngineProfile,
        name: Option<String>,
        values: &[ParameterChange],
    ) -> Result<usize, ValidationError> {
        self.slot(number)?;
        let mut part = Part::initialized(number, engine);
        part.name = name;
        let layout = engine.layout();
        let mut loaded = 0;
        for change in values {
            let valid = change.location.part_number() == number
                && engine
                    .parameter(change.parameter)
                    .is_some_and(|definition| definition.validate(change, layout).is_ok());
            if valid {
                part.values.insert(
                    (change.parameter, change.location.element_number()),
                    change.value,
                );
                loaded += 1;
            }
        }
        self.parts[usize::from(number - 1)] = Some(part);
        Ok(loaded)
    }

    pub fn store(
        &mut self,
        engine: &dyn EngineProfile,
        change: ParameterChange,
    ) -> Result<(), EditError> {
        self.part_for(change.location.part_number(), engine)?;
        let definition = engine
            .parameter(change.parameter)
            .ok_or(ValidationError::UnknownParameter(change.parameter))?;
        definition.validate(&change, engine.layout())?;
        self.insert(change);
        Ok(())
    }

    pub fn apply(
        &mut self,
        engine: &dyn EngineProfile,
        change: ParameterChange,
    ) -> Result<SysExMessage, EditError> {
        let part = self.part_for(change.location.part_number(), engine)?;
        let definition = engine
            .parameter(change.parameter)
            .ok_or(ValidationError::UnknownParameter(change.parameter))?;
        let effective = engine
            .refine(definition, part)
            .ok_or(EditError::Unavailable(change.parameter))?;
        effective.validate(&change, engine.layout())?;
        let message = engine.encode(&change)?;
        self.insert(change);
        Ok(message)
    }

    fn part_for(&self, number: u8, engine: &dyn EngineProfile) -> Result<&Part, EditError> {
        let part = if number == 0 {
            self.common.as_ref().ok_or(EditError::NoPerformance)?
        } else {
            self.slot(number)?
                .as_ref()
                .ok_or(EditError::EmptyPart(number))?
        };
        if part.engine == engine.id() {
            Ok(part)
        } else {
            Err(EditError::EngineMismatch {
                part: number,
                expected: engine.id(),
                actual: part.engine,
            })
        }
    }

    fn insert(&mut self, change: ParameterChange) {
        let part = match change.location.part_number().checked_sub(1) {
            None => self.common.as_mut(),
            Some(index) => self
                .parts
                .get_mut(usize::from(index))
                .and_then(Option::as_mut),
        };
        if let Some(part) = part {
            part.values.insert(
                (change.parameter, change.location.element_number()),
                change.value,
            );
        }
    }

    fn slot(&self, number: u8) -> Result<&Option<Part>, ValidationError> {
        number
            .checked_sub(1)
            .and_then(|index| self.parts.get(usize::from(index)))
            .ok_or(ValidationError::PartOutOfRange {
                part: number,
                max: self.part_count(),
            })
    }
}
