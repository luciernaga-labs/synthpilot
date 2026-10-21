use std::cmp::Ordering;
use std::fmt;

use super::{EngineId, Layout, ParameterChange, ParameterId, ValidationError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueRange {
    min: i32,
    max: i32,
}

impl ValueRange {
    pub const fn new(min: i32, max: i32) -> Self {
        assert!(min <= max, "range minimum must not exceed its maximum");
        Self { min, max }
    }

    pub const fn min(self) -> i32 {
        self.min
    }

    pub const fn max(self) -> i32 {
        self.max
    }

    pub const fn contains(self, value: i32) -> bool {
        value >= self.min && value <= self.max
    }

    pub const fn span(self) -> u32 {
        self.max.abs_diff(self.min)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Unit {
    Semitones,
    Decibels,
}

impl Unit {
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Semitones => "st",
            Self::Decibels => "dB",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListedValue {
    pub name: &'static str,
    pub group: &'static str,
}

/// A heading in a long list of choices, such as the effect categories of the
/// Effect Type List. It applies to the choices from `first` until the next group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChoiceGroup {
    pub name: &'static str,
    pub first: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueFormat {
    Number,
    Signed,
    Pan,
    Choice(&'static [&'static str]),
    Scaled {
        thousandths: u32,
        decimals: u8,
    },
    Listed {
        first: i32,
        values: &'static [ListedValue],
    },
    Note,
}

impl ValueFormat {
    pub fn format(self, value: i32) -> String {
        match self {
            Self::Signed if value > 0 => format!("+{value}"),
            Self::Number | Self::Signed => value.to_string(),
            Self::Pan => match value.cmp(&0) {
                Ordering::Less => format!("L{}", value.unsigned_abs()),
                Ordering::Equal => "C".to_owned(),
                Ordering::Greater => format!("R{value}"),
            },
            Self::Choice(labels) => usize::try_from(value)
                .ok()
                .and_then(|index| labels.get(index))
                .map_or_else(|| value.to_string(), |label| (*label).to_owned()),
            Self::Scaled {
                thousandths,
                decimals,
            } => scaled(value, thousandths, decimals),
            Self::Listed { first, values } => usize::try_from(value - first)
                .ok()
                .and_then(|index| values.get(index))
                .map_or_else(
                    || value.to_string(),
                    |listed| format!("{value} {}", listed.name),
                ),
            Self::Note => note(value),
        }
    }

    pub fn listed(self, value: i32) -> Option<ListedValue> {
        match self {
            Self::Listed { first, values } => usize::try_from(value - first)
                .ok()
                .and_then(|index| values.get(index))
                .copied(),
            _ => None,
        }
    }
}

const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

fn note(value: i32) -> String {
    let name = NOTE_NAMES[usize::try_from(value.rem_euclid(12)).unwrap_or_default()];
    format!("{name}{}", value.div_euclid(12) - 2)
}

fn scaled(value: i32, thousandths: u32, decimals: u8) -> String {
    let total = i64::from(value) * i64::from(thousandths);
    let divisor = 10_i64.pow(3 - u32::from(decimals.min(3)));
    let rounded = (total.abs() + divisor / 2) / divisor;
    let sign = match total.cmp(&0) {
        Ordering::Less => "-",
        Ordering::Equal => "",
        Ordering::Greater => "+",
    };
    let unit = 10_i64.pow(u32::from(decimals.min(3)));
    if decimals == 0 {
        format!("{sign}{rounded}")
    } else {
        let width = usize::from(decimals.min(3));
        format!("{sign}{}.{:0width$}", rounded / unit, rounded % unit)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    Performance,
    Part,
    Element,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceReference {
    pub document: &'static str,
    pub page: u16,
    pub table: &'static str,
}

impl fmt::Display for SourceReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, p. {}, {}", self.document, self.page, self.table)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Support {
    Implemented(SourceReference),
    NotImplemented { reason: &'static str },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParameterDefinition {
    pub id: ParameterId,
    pub name: &'static str,
    pub engine: EngineId,
    pub category: &'static str,
    pub scope: Scope,
    pub range: ValueRange,
    pub default: Option<i32>,
    pub unit: Option<Unit>,
    pub format: ValueFormat,
    pub support: Support,
    pub help: Option<&'static str>,
    pub groups: &'static [ChoiceGroup],
}

impl ParameterDefinition {
    pub const fn new(
        id: ParameterId,
        name: &'static str,
        engine: EngineId,
        category: &'static str,
        scope: Scope,
        range: ValueRange,
    ) -> Self {
        Self {
            id,
            name,
            engine,
            category,
            scope,
            range,
            default: None,
            unit: None,
            format: ValueFormat::Number,
            support: Support::NotImplemented {
                reason: "SynthPilot can't edit this parameter yet.",
            },
            help: None,
            groups: &[],
        }
    }

    #[must_use]
    pub const fn with_default(mut self, value: i32) -> Self {
        assert!(
            self.range.contains(value),
            "default value must be inside the parameter range"
        );
        self.default = Some(value);
        self
    }

    #[must_use]
    pub const fn with_unit(mut self, unit: Unit) -> Self {
        self.unit = Some(unit);
        self
    }

    #[must_use]
    pub const fn with_format(mut self, format: ValueFormat) -> Self {
        self.format = format;
        self
    }

    #[must_use]
    pub const fn with_help(mut self, help: &'static str) -> Self {
        self.help = Some(help);
        self
    }

    #[must_use]
    pub const fn with_groups(mut self, groups: &'static [ChoiceGroup]) -> Self {
        self.groups = groups;
        self
    }

    /// The group heading that starts at `value`, if any.
    pub fn group_starting_at(&self, value: i32) -> Option<&'static str> {
        self.groups
            .iter()
            .find(|group| group.first == value)
            .map(|group| group.name)
    }

    /// The group `value` belongs to, if the choices are grouped.
    pub fn group_of(&self, value: i32) -> Option<&'static str> {
        self.groups
            .iter()
            .rev()
            .find(|group| group.first <= value)
            .map(|group| group.name)
    }

    #[must_use]
    pub const fn with_support(mut self, support: Support) -> Self {
        self.support = support;
        self
    }

    pub const fn is_implemented(&self) -> bool {
        matches!(self.support, Support::Implemented(_))
    }

    pub fn display_value(&self, value: i32) -> String {
        let text = self.format.format(value);
        match self.unit {
            Some(unit) => format!("{text} {}", unit.suffix()),
            None => text,
        }
    }

    pub fn value_named(&self, text: &str) -> Option<i32> {
        let wanted = text.trim();
        (self.range.min()..=self.range.max()).find(|value| {
            self.format.format(*value).eq_ignore_ascii_case(wanted)
                || self
                    .format
                    .listed(*value)
                    .is_some_and(|listed| listed.name.eq_ignore_ascii_case(wanted))
        })
    }

    pub fn display_range(&self) -> String {
        format!(
            "{} … {}",
            self.display_value(self.range.min()),
            self.display_value(self.range.max())
        )
    }

    pub fn validate_value(&self, value: i32) -> Result<(), ValidationError> {
        if self.range.contains(value) {
            Ok(())
        } else {
            Err(ValidationError::OutOfRange {
                parameter: self.id,
                value,
                min: self.range.min(),
                max: self.range.max(),
            })
        }
    }

    pub fn validate(
        &self,
        change: &ParameterChange,
        layout: Layout,
    ) -> Result<(), ValidationError> {
        layout.check(self, change.location)?;
        self.validate_value(change.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_values_for_humans() {
        assert_eq!(ValueFormat::Signed.format(5), "+5");
        assert_eq!(ValueFormat::Signed.format(-5), "-5");
        assert_eq!(ValueFormat::Pan.format(-63), "L63");
        assert_eq!(ValueFormat::Pan.format(0), "C");
        assert_eq!(ValueFormat::Pan.format(12), "R12");
        assert_eq!(ValueFormat::Choice(&["Off", "On"]).format(1), "On");
        assert_eq!(ValueFormat::Choice(&["Off", "On"]).format(7), "7");
    }

    #[test]
    fn formats_listed_values() {
        const WAVES: &[ListedValue] = &[
            ListedValue {
                name: "EP1 Sw",
                group: "Keys/Rd",
            },
            ListedValue {
                name: "DX1",
                group: "Keys/FM",
            },
        ];
        let format = ValueFormat::Listed {
            first: 1,
            values: WAVES,
        };
        assert_eq!(format.format(1), "1 EP1 Sw");
        assert_eq!(format.format(2), "2 DX1");
        assert_eq!(format.format(3), "3");
        assert_eq!(format.format(0), "0");
        assert_eq!(format.listed(2).map(|value| value.group), Some("Keys/FM"));
        assert_eq!(format.listed(9), None);
        assert_eq!(ValueFormat::Number.listed(1), None);
    }

    #[test]
    fn formats_notes_from_c_minus_2() {
        assert_eq!(ValueFormat::Note.format(0), "C-2");
        assert_eq!(ValueFormat::Note.format(60), "C3");
        assert_eq!(ValueFormat::Note.format(61), "C#3");
        assert_eq!(ValueFormat::Note.format(127), "G8");
    }

    #[test]
    fn values_are_found_by_their_label() {
        let note = level().with_format(ValueFormat::Note);
        assert_eq!(note.value_named("c3"), Some(60));
        assert_eq!(note.value_named(" G8 "), Some(127));
        assert_eq!(note.value_named("H2"), None);
        let switch = level().with_format(ValueFormat::Choice(&["Off", "On"]));
        assert_eq!(switch.value_named("on"), Some(1));
    }

    #[test]
    fn formats_scaled_values() {
        let eq = ValueFormat::Scaled {
            thousandths: 375,
            decimals: 2,
        };
        assert_eq!(eq.format(-32), "-12.00");
        assert_eq!(eq.format(0), "0.00");
        assert_eq!(eq.format(1), "+0.38");
        assert_eq!(eq.format(32), "+12.00");
        let whole = ValueFormat::Scaled {
            thousandths: 500,
            decimals: 0,
        };
        assert_eq!(whole.format(3), "+2");
        assert_eq!(whole.format(-4), "-2");
    }

    #[test]
    fn appends_units() {
        let definition = ParameterDefinition::new(
            ParameterId::new("tune"),
            "Tune",
            EngineId::new("engine"),
            "Pitch",
            Scope::Part,
            ValueRange::new(-48, 48),
        )
        .with_format(ValueFormat::Signed)
        .with_unit(Unit::Semitones);
        assert_eq!(definition.display_value(12), "+12 st");
        assert_eq!(definition.display_range(), "-48 st … +48 st");
    }

    fn level() -> ParameterDefinition {
        ParameterDefinition::new(
            ParameterId::new("level"),
            "Level",
            EngineId::new("engine"),
            "Amplitude",
            Scope::Element,
            ValueRange::new(0, 127),
        )
        .with_default(100)
    }

    #[test]
    fn help_is_optional() {
        assert_eq!(level().help, None);
        let help = "Volume of this Element.";
        assert_eq!(level().with_help(help).help, Some(help));
    }

    #[test]
    fn ranges_report_their_bounds() {
        let range = ValueRange::new(-64, 63);
        assert_eq!((range.min(), range.max(), range.span()), (-64, 63, 127));
        assert!(range.contains(0) && !range.contains(64));
    }

    #[test]
    fn validates_values_against_the_range() {
        let definition = level();
        assert_eq!(definition.default, Some(100));
        assert!(!definition.is_implemented());
        assert_eq!(definition.validate_value(127), Ok(()));
        assert_eq!(
            definition.validate_value(128),
            Err(ValidationError::OutOfRange {
                parameter: ParameterId::new("level"),
                value: 128,
                min: 0,
                max: 127,
            })
        );
    }

    #[test]
    #[should_panic(expected = "inside the parameter range")]
    fn defaults_must_be_in_range() {
        let _ = level().with_default(200);
    }

    #[test]
    fn source_references_read_naturally() {
        let source = SourceReference {
            document: "Data List",
            page: 217,
            table: "Normal Part Element",
        };
        assert_eq!(source.to_string(), "Data List, p. 217, Normal Part Element");
        let definition = level().with_support(Support::Implemented(source));
        assert!(definition.is_implemented());
    }
}
