use super::{ParameterDefinition, ParameterId, Scope, ValidationError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Location {
    part: u8,
    element: Option<u8>,
}

impl Location {
    pub const fn performance() -> Self {
        Self {
            part: 0,
            element: None,
        }
    }

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
            Scope::Performance => Self::performance(),
            Scope::Part => Self::part(part),
            Scope::Element => Self::element(part, element),
        }
    }

    pub const fn is_performance(self) -> bool {
        self.part == 0
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
        if definition.scope == Scope::Performance {
            return if location == Location::performance() {
                Ok(())
            } else {
                Err(ValidationError::PerformanceOnly(definition.id))
            };
        }
        if !(1..=self.parts).contains(&location.part) {
            return Err(ValidationError::PartOutOfRange {
                part: location.part,
                max: self.parts,
            });
        }
        match (definition.scope, location.element) {
            (Scope::Performance, _) | (Scope::Part, None) => Ok(()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{EngineId, ValueRange};

    const LAYOUT: Layout = Layout {
        parts: 16,
        elements: 8,
    };

    fn definition(scope: Scope) -> ParameterDefinition {
        ParameterDefinition::new(
            ParameterId::new("test"),
            "Test",
            EngineId::new("engine"),
            "Category",
            scope,
            ValueRange::new(0, 127),
        )
    }

    #[test]
    fn performance_locations_have_no_part() {
        let location = Location::for_scope(Scope::Performance, 3, 2);
        assert_eq!(location, Location::performance());
        assert!(location.is_performance());
        assert_eq!(location.element_number(), None);
        assert!(!Location::part(1).is_performance());
    }

    #[test]
    fn performance_parameters_only_accept_the_performance() {
        let performance = definition(Scope::Performance);
        let id = ParameterId::new("test");
        assert_eq!(LAYOUT.check(&performance, Location::performance()), Ok(()));
        assert_eq!(
            LAYOUT.check(&performance, Location::part(1)),
            Err(ValidationError::PerformanceOnly(id))
        );
        assert_eq!(
            LAYOUT.check(&definition(Scope::Part), Location::performance()),
            Err(ValidationError::PartOutOfRange { part: 0, max: 16 })
        );
    }

    #[test]
    fn part_and_element_scopes_check_their_location() {
        let id = ParameterId::new("test");
        let part = definition(Scope::Part);
        let element = definition(Scope::Element);
        assert_eq!(LAYOUT.check(&part, Location::part(16)), Ok(()));
        assert_eq!(
            LAYOUT.check(&part, Location::element(1, 1)),
            Err(ValidationError::UnexpectedElement(id))
        );
        assert_eq!(
            LAYOUT.check(&element, Location::part(1)),
            Err(ValidationError::MissingElement(id))
        );
        assert_eq!(
            LAYOUT.check(&element, Location::element(1, 9)),
            Err(ValidationError::ElementOutOfRange { element: 9, max: 8 })
        );
    }
}
