use super::{DATA_LIST, MODEL_ID};
use crate::domain::{
    EncodeError, Layout, Location, ParameterChange, ParameterDefinition, SourceReference, Support,
    ValidationError,
};
use crate::protocol::SysExMessage;
use crate::protocol::yamaha::{self, Address, DeviceNumber};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Addressing {
    Part,
    Element,
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
            page,
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
        let part = location.part_number() - 1;
        let mid = match (self.block.addressing, location.element_number()) {
            (Addressing::Part, _) => part,
            (Addressing::Element, Some(element)) => ((element - 1) << 4) | part,
            (Addressing::Element, None) => {
                return Err(ValidationError::MissingElement(definition.id).into());
            }
        };
        Ok(Address::new(self.block.high, mid, self.low)?)
    }

    fn location(&self, mid: u8) -> Location {
        match self.block.addressing {
            Addressing::Part => Location::part(mid + 1),
            Addressing::Element => Location::element((mid & 0x0F) + 1, (mid >> 4) + 1),
        }
    }

    fn value(&self, definition: &ParameterDefinition, raw: u32) -> Option<i32> {
        if !(self.raw_min..=self.raw_max).contains(&raw) {
            return None;
        }
        let steps = i32::try_from(raw - self.raw_min).ok()?;
        Some(definition.range.min() + steps)
    }

    pub(super) fn data(
        &self,
        definition: &ParameterDefinition,
        value: i32,
    ) -> Result<Vec<u8>, EncodeError> {
        let offset = value.abs_diff(definition.range.min());
        Ok(yamaha::encode_data(self.raw_min + offset, self.size)?)
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
            mapping.raw_max - mapping.raw_min == definition.range.span(),
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
            if mapping.block.high != address.high() {
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
        let mapping = Mapping::new(block(Addressing::Part), 0x4F, 1, (0x00, 0x7F), 220);
        let address = mapping.address(&definition(Scope::Part), Location::part(16));
        assert_eq!(address.map(Address::to_bytes), Ok([0x41, 0x0F, 0x4F]));
    }
}
