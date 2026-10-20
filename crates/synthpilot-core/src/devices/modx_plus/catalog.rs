use super::{DATA_LIST, MODEL_ID};
use crate::domain::{
    EncodeError, Layout, Location, ParameterChange, ParameterDefinition, SourceReference, Support,
    ValidationError,
};
use crate::protocol::SysExMessage;
use crate::protocol::yamaha::{self, Address, DeviceNumber};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Addressing {
    Performance { mid: u8 },
    Part { base: u8 },
    Element,
}

impl Addressing {
    pub(super) const PART: Self = Self::Part { base: 0x00 };

    const fn accepts(self, mid: u8) -> bool {
        match self {
            Self::Performance { mid: expected } => mid == expected,
            Self::Part { base } => mid & 0xF0 == base,
            Self::Element => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Block {
    high: u8,
    addressing: Addressing,
    table: &'static str,
}

impl Block {
    pub(super) const fn new(high: u8, addressing: Addressing, table: &'static str) -> Self {
        Self {
            high,
            addressing,
            table,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Mapping {
    block: Block,
    low: u8,
    size: usize,
    raw_min: u32,
    raw_max: u32,
    table: Option<&'static [u32]>,
    page: u16,
}

impl Mapping {
    pub(super) const fn new(
        block: Block,
        low: u8,
        size: usize,
        raw: (u32, u32),
        page: u16,
    ) -> Self {
        assert!(low <= 0x7F, "address low byte must be 7-bit");
        assert!(size >= 1 && size <= 2, "only 1- and 2-byte data is mapped");
        assert!(raw.0 <= raw.1, "raw range must be ordered");
        assert!(
            raw.1 >> (7 * size) == 0,
            "raw maximum must fit in the data size"
        );
        Self {
            block,
            low,
            size,
            raw_min: raw.0,
            raw_max: raw.1,
            table: None,
            page,
        }
    }

    pub(super) const fn listed(
        block: Block,
        low: u8,
        size: usize,
        table: &'static [u32],
        page: u16,
    ) -> Self {
        assert!(!table.is_empty(), "a value table needs at least one value");
        let mut min = table[0];
        let mut max = table[0];
        let mut index = 1;
        while index < table.len() {
            let mut earlier = 0;
            while earlier < index {
                assert!(
                    table[earlier] != table[index],
                    "table values must be unique"
                );
                earlier += 1;
            }
            if table[index] < min {
                min = table[index];
            }
            if table[index] > max {
                max = table[index];
            }
            index += 1;
        }
        let mut mapping = Self::new(block, low, size, (min, max), page);
        mapping.table = Some(table);
        mapping
    }

    const fn steps(&self) -> u32 {
        match self.table {
            Some(table) => {
                let mut steps = 0;
                let mut index = 1;
                while index < table.len() {
                    steps += 1;
                    index += 1;
                }
                steps
            }
            None => self.raw_max - self.raw_min,
        }
    }

    pub(super) const fn source(&self) -> SourceReference {
        SourceReference {
            document: DATA_LIST,
            page: self.page,
            table: self.block.table,
        }
    }

    pub(super) fn address(
        &self,
        definition: &ParameterDefinition,
        location: Location,
    ) -> Result<Address, EncodeError> {
        let part = location.part_number().wrapping_sub(1);
        let mid = match (self.block.addressing, location.element_number()) {
            (Addressing::Performance { mid }, _) => mid,
            (Addressing::Part { base }, _) => base | part,
            (Addressing::Element, Some(element)) => ((element - 1) << 4) | part,
            (Addressing::Element, None) => {
                return Err(ValidationError::MissingElement(definition.id).into());
            }
        };
        Ok(Address::new(self.block.high, mid, self.low)?)
    }

    fn location(&self, mid: u8) -> Location {
        match self.block.addressing {
            Addressing::Performance { .. } => Location::performance(),
            Addressing::Part { .. } => Location::part((mid & 0x0F) + 1),
            Addressing::Element => Location::element((mid & 0x0F) + 1, (mid >> 4) + 1),
        }
    }

    fn value(&self, definition: &ParameterDefinition, raw: u32) -> Option<i32> {
        let steps = match self.table {
            Some(table) => table.iter().position(|value| *value == raw)?,
            None if (self.raw_min..=self.raw_max).contains(&raw) => {
                usize::try_from(raw - self.raw_min).ok()?
            }
            None => return None,
        };
        Some(definition.range.min() + i32::try_from(steps).ok()?)
    }

    pub(super) fn data(
        &self,
        definition: &ParameterDefinition,
        value: i32,
    ) -> Result<Vec<u8>, EncodeError> {
        let offset = value.abs_diff(definition.range.min());
        let raw = match self.table {
            Some(table) => usize::try_from(offset)
                .ok()
                .and_then(|index| table.get(index).copied())
                .ok_or(ValidationError::OutOfRange {
                    parameter: definition.id,
                    value,
                    min: definition.range.min(),
                    max: definition.range.max(),
                })?,
            None => self.raw_min + offset,
        };
        Ok(yamaha::encode_data(raw, self.size)?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Entry {
    pub(super) definition: ParameterDefinition,
    pub(super) mapping: Option<Mapping>,
}

impl Entry {
    pub(super) const fn verified(definition: ParameterDefinition, mapping: Mapping) -> Self {
        assert!(
            mapping.steps() == definition.range.span(),
            "raw range and display range must have the same number of steps"
        );
        let definition = definition.with_support(Support::Implemented(mapping.source()));
        Self {
            definition,
            mapping: Some(mapping),
        }
    }

    pub(super) const fn unverified(definition: ParameterDefinition, reason: &'static str) -> Self {
        Self {
            definition: definition.with_support(Support::NotImplemented { reason }),
            mapping: None,
        }
    }
}

pub(super) fn definitions(entries: &[Entry]) -> Vec<ParameterDefinition> {
    entries.iter().map(|entry| entry.definition).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Span {
    Block,
    Single,
}

pub(super) fn decode(
    entries: &[Entry],
    layout: Layout,
    address: Address,
    data: &[u8],
    span: Span,
) -> Vec<ParameterChange> {
    entries
        .iter()
        .filter_map(|entry| {
            let mapping = entry.mapping?;
            if mapping.block.high != address.high()
                || !mapping.block.addressing.accepts(address.mid())
            {
                return None;
            }
            let start = usize::from(mapping.low.checked_sub(address.low())?);
            if span == Span::Single && (start != 0 || data.len() != mapping.size) {
                return None;
            }
            let bytes = data.get(start..start + mapping.size)?;
            let raw = yamaha::decode_data(bytes).ok()?;
            let value = mapping.value(&entry.definition, raw)?;
            let change =
                ParameterChange::new(entry.definition.id, mapping.location(address.mid()), value);
            entry.definition.validate(&change, layout).ok()?;
            Some(change)
        })
        .collect()
}

pub(super) fn encode(
    entries: &[Entry],
    layout: Layout,
    device_number: DeviceNumber,
    change: &ParameterChange,
) -> Result<SysExMessage, EncodeError> {
    let entry = entries
        .iter()
        .find(|entry| entry.definition.id == change.parameter)
        .ok_or(ValidationError::UnknownParameter(change.parameter))?;
    let mapping = entry
        .mapping
        .ok_or(EncodeError::NotImplemented(change.parameter))?;
    entry.definition.validate(change, layout)?;
    let address = mapping.address(&entry.definition, change.location)?;
    let data = mapping.data(&entry.definition, change.value)?;
    Ok(yamaha::parameter_change(
        device_number,
        MODEL_ID,
        address,
        &data,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{EngineId, ParameterId, Scope, ValueRange};

    fn block(addressing: Addressing) -> Block {
        Block::new(0x41, addressing, "Test table")
    }

    fn definition(scope: Scope) -> ParameterDefinition {
        ParameterDefinition::new(
            ParameterId::new("test"),
            "Test",
            EngineId::new("engine"),
            "Category",
            scope,
            ValueRange::new(-64, 63),
        )
    }

    #[test]
    fn verified_entries_cite_their_source() {
        let mapping = Mapping::new(block(Addressing::Element), 0x29, 1, (0x00, 0x7F), 217);
        let entry = Entry::verified(definition(Scope::Element), mapping);
        assert_eq!(
            entry.definition.support,
            Support::Implemented(SourceReference {
                document: DATA_LIST,
                page: 217,
                table: "Test table",
            })
        );
        assert_eq!(entry.mapping, Some(mapping));
    }

    #[test]
    fn unverified_entries_keep_their_reason() {
        let entry = Entry::unverified(definition(Scope::Element), "Not documented.");
        assert_eq!(
            entry.definition.support,
            Support::NotImplemented {
                reason: "Not documented."
            }
        );
        assert!(entry.mapping.is_none());
    }

    #[test]
    #[should_panic(expected = "same number of steps")]
    fn mismatched_ranges_are_rejected() {
        let mapping = Mapping::new(block(Addressing::Element), 0x00, 1, (0x00, 0x0A), 217);
        let _ = Entry::verified(definition(Scope::Element), mapping);
    }

    #[test]
    #[should_panic(expected = "fit in the data size")]
    fn raw_ranges_must_fit_the_data_size() {
        let _ = Mapping::new(block(Addressing::Element), 0x00, 1, (0x00, 0xFF), 217);
    }

    #[test]
    fn element_addresses_combine_element_and_part() {
        let mapping = Mapping::new(block(Addressing::Element), 0x28, 1, (0x00, 0x7F), 217);
        let address = mapping.address(&definition(Scope::Element), Location::element(3, 5));
        assert_eq!(address.map(Address::to_bytes), Ok([0x41, 0x42, 0x28]));
    }

    #[test]
    fn element_addresses_need_an_element() {
        let mapping = Mapping::new(block(Addressing::Element), 0x28, 1, (0x00, 0x7F), 217);
        let id = ParameterId::new("test");
        assert_eq!(
            mapping.address(&definition(Scope::Element), Location::part(1)),
            Err(ValidationError::MissingElement(id).into())
        );
    }

    #[test]
    fn part_addresses_use_the_part_number() {
        let mapping = Mapping::new(block(Addressing::PART), 0x4F, 1, (0x00, 0x7F), 220);
        let address = mapping.address(&definition(Scope::Part), Location::part(16));
        assert_eq!(address.map(Address::to_bytes), Ok([0x41, 0x0F, 0x4F]));
    }

    #[test]
    fn offset_part_blocks_add_their_base() {
        let mapping = Mapping::new(
            block(Addressing::Part { base: 0x20 }),
            0x00,
            1,
            (0x00, 0x7F),
            209,
        );
        let address = mapping.address(&definition(Scope::Part), Location::part(3));
        assert_eq!(address.map(Address::to_bytes), Ok([0x41, 0x22, 0x00]));
        assert_eq!(mapping.location(0x22), Location::part(3));
    }

    #[test]
    fn performance_blocks_use_a_fixed_address() {
        let mapping = Mapping::new(
            block(Addressing::Performance { mid: 0x42 }),
            0x44,
            1,
            (0x00, 0x7F),
            198,
        );
        let address = mapping.address(&definition(Scope::Performance), Location::performance());
        assert_eq!(address.map(Address::to_bytes), Ok([0x41, 0x42, 0x44]));
        assert_eq!(mapping.location(0x42), Location::performance());
        assert!(Addressing::Performance { mid: 0x42 }.accepts(0x42));
        assert!(!Addressing::Performance { mid: 0x42 }.accepts(0x41));
        assert!(Addressing::Part { base: 0x30 }.accepts(0x3F));
        assert!(!Addressing::Part { base: 0x30 }.accepts(0x0F));
    }

    const TABLE: &[u32] = &[0x0000, 0x0083, 0x0080];

    fn listed() -> (ParameterDefinition, Mapping) {
        let definition = ParameterDefinition::new(
            ParameterId::new("type"),
            "Type",
            EngineId::new("engine"),
            "Category",
            Scope::Part,
            ValueRange::new(0, 2),
        );
        let mapping = Mapping::listed(block(Addressing::PART), 0x00, 2, TABLE, 144);
        (definition, mapping)
    }

    #[test]
    fn listed_values_translate_through_the_table() {
        let (definition, mapping) = listed();
        assert_eq!((mapping.raw_min, mapping.raw_max), (0x0000, 0x0083));
        assert_eq!(mapping.steps(), 2);
        assert_eq!(mapping.data(&definition, 1), Ok(vec![0x01, 0x03]));
        assert_eq!(mapping.data(&definition, 2), Ok(vec![0x01, 0x00]));
        assert_eq!(mapping.value(&definition, 0x0080), Some(2));
        assert_eq!(mapping.value(&definition, 0x0081), None);
        assert!(mapping.data(&definition, 3).is_err());
        let entry = Entry::verified(definition, mapping);
        assert!(entry.definition.is_implemented());
    }

    #[test]
    fn linear_values_outside_the_raw_range_are_ignored() {
        let mapping = Mapping::new(block(Addressing::PART), 0x00, 1, (0x10, 0x20), 207);
        let definition = definition(Scope::Part);
        assert_eq!(mapping.value(&definition, 0x0F), None);
        assert_eq!(mapping.value(&definition, 0x10), Some(-64));
    }

    #[test]
    #[should_panic(expected = "unique")]
    fn table_values_must_be_unique() {
        let _ = Mapping::listed(block(Addressing::PART), 0x00, 2, &[1, 2, 1], 144);
    }
}
