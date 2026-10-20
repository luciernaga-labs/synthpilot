use super::{ParameterDefinition, ParameterId, Scope, ValidationError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Location {
    part: u8,
    element: Option<u8>,
}

impl Location {
    pub const fn part(part: u8) -> Self {
        Self {
            part,
            element: None,
        }
    }

    pub const fn element(part: u8, element: u8) -> Self {
        Self {
            part,
            element: Some(element),
        }
    }

    pub const fn for_scope(scope: Scope, part: u8, element: u8) -> Self {
        match scope {
            Scope::Part => Self::part(part),
            Scope::Element => Self::element(part, element),
        }
    }

    pub const fn part_number(self) -> u8 {
        self.part
    }

    pub const fn element_number(self) -> Option<u8> {
        self.element
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Layout {
    pub parts: u8,
    pub elements: u8,
}

impl Layout {
    pub fn check(
        self,
        definition: &ParameterDefinition,
        location: Location,
    ) -> Result<(), ValidationError> {
        if !(1..=self.parts).contains(&location.part) {
            return Err(ValidationError::PartOutOfRange {
                part: location.part,
                max: self.parts,
            });
        }
        match (definition.scope, location.element) {
            (Scope::Part, None) => Ok(()),
            (Scope::Part, Some(_)) => Err(ValidationError::UnexpectedElement(definition.id)),
            (Scope::Element, None) => Err(ValidationError::MissingElement(definition.id)),
            (Scope::Element, Some(element)) if (1..=self.elements).contains(&element) => Ok(()),
            (Scope::Element, Some(element)) => Err(ValidationError::ElementOutOfRange {
                element,
                max: self.elements,
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParameterChange {
    pub parameter: ParameterId,
    pub location: Location,
    pub value: i32,
}

impl ParameterChange {
    pub const fn new(parameter: ParameterId, location: Location, value: i32) -> Self {
        Self {
            parameter,
            location,
            value,
        }
    }
}
