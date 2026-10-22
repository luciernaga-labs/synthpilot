use std::collections::HashMap;
use std::sync::OnceLock;

use super::catalog::{Block, Entry, Mapping};
use crate::domain::{EngineId, ParameterDefinition, ParameterId, Part, Scope, Support, ValueRange};

pub const SLOTS: usize = 24;

const TABLE: &str = include_str!("effect_parameters.tsv");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum List {
    Reverb,
    Variation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    pub name: &'static str,
    pub min: i32,
    pub max: i32,
    pub range: &'static str,
    pub page: u16,
}

type Slots = HashMap<(List, u32, usize), Slot>;

static PARSED: OnceLock<Slots> = OnceLock::new();

fn parsed() -> &'static Slots {
    PARSED.get_or_init(|| TABLE.lines().filter_map(parse).collect())
}

fn parse(line: &'static str) -> Option<((List, u32, usize), Slot)> {
    let mut columns = line.split('\t');
    let list = match columns.next()? {
        "rev" => List::Reverb,
        "var" => List::Variation,
        _ => return None,
    };
    let kind = u32::from_str_radix(columns.next()?, 16).ok()?;
    let slot = columns.next()?.parse().ok()?;
    let name = columns.next()?;
    let min = columns.next()?.parse().ok()?;
    let max = columns.next()?.parse().ok()?;
    let range = columns.next()?;
    let page = columns.next()?.parse().ok()?;
    Some((
        (list, kind, slot),
        Slot {
            name,
            min,
            max,
            range,
            page,
        },
    ))
}

pub(super) fn slot(list: List, kind: u32, number: usize) -> Option<Slot> {
    parsed().get(&(list, kind, number)).copied()
}

pub(super) struct Group {
    pub(super) list: List,
    pub(super) control: ParameterId,
    pub(super) kinds: &'static [u32],
    pub(super) ids: &'static [ParameterId; SLOTS],
    pub(super) names: &'static [&'static str; SLOTS],
    pub(super) block: Block,
    pub(super) category: &'static str,
    pub(super) scope: Scope,
    pub(super) pages: (u16, u16),
}

impl Group {
    pub(super) fn entries(&self, engine: EngineId) -> Vec<Entry> {
        (0..SLOTS)
            .map(|index| {
                let low = u8::try_from(3 + 2 * index).unwrap_or(u8::MAX);
                let page = if index < 22 {
                    self.pages.0
                } else {
                    self.pages.1
                };
                Entry::verified(
                    ParameterDefinition::new(
                        self.ids[index],
                        self.names[index],
                        engine,
                        self.category,
                        self.scope,
                        ValueRange::new(0, 0x3FFF),
                    ),
                    Mapping::new(self.block, low, 2, (0, 0x3FFF), page),
                )
            })
            .collect()
    }

    fn position(&self, id: ParameterId) -> Option<usize> {
        self.ids.iter().position(|candidate| *candidate == id)
    }

    fn kind(&self, value: i32) -> Option<u32> {
        usize::try_from(value)
            .ok()
            .and_then(|index| self.kinds.get(index))
            .copied()
    }

    fn shaped(
        &self,
        parameter: &ParameterDefinition,
        kind: u32,
        index: usize,
    ) -> Option<ParameterDefinition> {
        let slot = slot(self.list, kind, index + 1)?;
        let mut definition = *parameter;
        definition.name = slot.name;
        definition.range = ValueRange::new(slot.min, slot.max);
        definition.default = None;
        definition.help = Some(slot.range);
        if let Support::Implemented(mut source) = definition.support {
            source.page = slot.page;
            source.table = "Effect Parameter List";
            definition.support = Support::Implemented(source);
        }
        Some(definition)
    }
}

pub(super) fn refine(
    groups: &[&Group],
    parameter: &ParameterDefinition,
    part: &Part,
) -> Option<ParameterDefinition> {
    for group in groups {
        if let Some(index) = group.position(parameter.id) {
            let kind = group.kind(part.value(group.control, None)?)?;
            return group.shaped(parameter, kind, index);
        }
    }
    Some(*parameter)
}

pub(super) fn dependents(
    groups: &[&Group],
    parameters: &[ParameterDefinition],
    control: &ParameterDefinition,
    value: i32,
) -> Vec<ParameterDefinition> {
    let Some(group) = groups.iter().find(|group| group.control == control.id) else {
        return Vec::new();
    };
    let Some(kind) = group.kind(value) else {
        return Vec::new();
    };
    group
        .ids
        .iter()
        .enumerate()
        .filter_map(|(index, id)| {
            let parameter = parameters.iter().find(|parameter| parameter.id == *id)?;
            group.shaped(parameter, kind, index)
        })
        .collect()
}

pub(super) const REVERB_IDS: [ParameterId; SLOTS] = [
    ParameterId::new("effects.reverb-parameter-1"),
    ParameterId::new("effects.reverb-parameter-2"),
    ParameterId::new("effects.reverb-parameter-3"),
    ParameterId::new("effects.reverb-parameter-4"),
    ParameterId::new("effects.reverb-parameter-5"),
    ParameterId::new("effects.reverb-parameter-6"),
    ParameterId::new("effects.reverb-parameter-7"),
    ParameterId::new("effects.reverb-parameter-8"),
    ParameterId::new("effects.reverb-parameter-9"),
    ParameterId::new("effects.reverb-parameter-10"),
    ParameterId::new("effects.reverb-parameter-11"),
    ParameterId::new("effects.reverb-parameter-12"),
    ParameterId::new("effects.reverb-parameter-13"),
    ParameterId::new("effects.reverb-parameter-14"),
    ParameterId::new("effects.reverb-parameter-15"),
    ParameterId::new("effects.reverb-parameter-16"),
    ParameterId::new("effects.reverb-parameter-17"),
    ParameterId::new("effects.reverb-parameter-18"),
    ParameterId::new("effects.reverb-parameter-19"),
    ParameterId::new("effects.reverb-parameter-20"),
    ParameterId::new("effects.reverb-parameter-21"),
    ParameterId::new("effects.reverb-parameter-22"),
    ParameterId::new("effects.reverb-parameter-23"),
    ParameterId::new("effects.reverb-parameter-24"),
];

pub(super) const REVERB_SLOT_NAMES: [&str; SLOTS] = [
    "Reverb Parameter 1",
    "Reverb Parameter 2",
    "Reverb Parameter 3",
    "Reverb Parameter 4",
    "Reverb Parameter 5",
    "Reverb Parameter 6",
    "Reverb Parameter 7",
    "Reverb Parameter 8",
    "Reverb Parameter 9",
    "Reverb Parameter 10",
    "Reverb Parameter 11",
    "Reverb Parameter 12",
    "Reverb Parameter 13",
    "Reverb Parameter 14",
    "Reverb Parameter 15",
    "Reverb Parameter 16",
    "Reverb Parameter 17",
    "Reverb Parameter 18",
    "Reverb Parameter 19",
    "Reverb Parameter 20",
    "Reverb Parameter 21",
    "Reverb Parameter 22",
    "Reverb Parameter 23",
    "Reverb Parameter 24",
];

pub(super) const VARIATION_IDS: [ParameterId; SLOTS] = [
    ParameterId::new("effects.variation-parameter-1"),
    ParameterId::new("effects.variation-parameter-2"),
    ParameterId::new("effects.variation-parameter-3"),
    ParameterId::new("effects.variation-parameter-4"),
    ParameterId::new("effects.variation-parameter-5"),
    ParameterId::new("effects.variation-parameter-6"),
    ParameterId::new("effects.variation-parameter-7"),
    ParameterId::new("effects.variation-parameter-8"),
    ParameterId::new("effects.variation-parameter-9"),
    ParameterId::new("effects.variation-parameter-10"),
    ParameterId::new("effects.variation-parameter-11"),
    ParameterId::new("effects.variation-parameter-12"),
    ParameterId::new("effects.variation-parameter-13"),
    ParameterId::new("effects.variation-parameter-14"),
    ParameterId::new("effects.variation-parameter-15"),
    ParameterId::new("effects.variation-parameter-16"),
    ParameterId::new("effects.variation-parameter-17"),
    ParameterId::new("effects.variation-parameter-18"),
    ParameterId::new("effects.variation-parameter-19"),
    ParameterId::new("effects.variation-parameter-20"),
    ParameterId::new("effects.variation-parameter-21"),
    ParameterId::new("effects.variation-parameter-22"),
    ParameterId::new("effects.variation-parameter-23"),
    ParameterId::new("effects.variation-parameter-24"),
];

pub(super) const VARIATION_SLOT_NAMES: [&str; SLOTS] = [
    "Variation Parameter 1",
    "Variation Parameter 2",
    "Variation Parameter 3",
    "Variation Parameter 4",
    "Variation Parameter 5",
    "Variation Parameter 6",
    "Variation Parameter 7",
    "Variation Parameter 8",
    "Variation Parameter 9",
    "Variation Parameter 10",
    "Variation Parameter 11",
    "Variation Parameter 12",
    "Variation Parameter 13",
    "Variation Parameter 14",
    "Variation Parameter 15",
    "Variation Parameter 16",
    "Variation Parameter 17",
    "Variation Parameter 18",
    "Variation Parameter 19",
    "Variation Parameter 20",
    "Variation Parameter 21",
    "Variation Parameter 22",
    "Variation Parameter 23",
    "Variation Parameter 24",
];

pub(super) const INSERTION_A_IDS: [ParameterId; SLOTS] = [
    ParameterId::new("part.insertion-a-parameter-1"),
    ParameterId::new("part.insertion-a-parameter-2"),
    ParameterId::new("part.insertion-a-parameter-3"),
    ParameterId::new("part.insertion-a-parameter-4"),
    ParameterId::new("part.insertion-a-parameter-5"),
    ParameterId::new("part.insertion-a-parameter-6"),
    ParameterId::new("part.insertion-a-parameter-7"),
    ParameterId::new("part.insertion-a-parameter-8"),
    ParameterId::new("part.insertion-a-parameter-9"),
    ParameterId::new("part.insertion-a-parameter-10"),
    ParameterId::new("part.insertion-a-parameter-11"),
    ParameterId::new("part.insertion-a-parameter-12"),
    ParameterId::new("part.insertion-a-parameter-13"),
    ParameterId::new("part.insertion-a-parameter-14"),
    ParameterId::new("part.insertion-a-parameter-15"),
    ParameterId::new("part.insertion-a-parameter-16"),
    ParameterId::new("part.insertion-a-parameter-17"),
    ParameterId::new("part.insertion-a-parameter-18"),
    ParameterId::new("part.insertion-a-parameter-19"),
    ParameterId::new("part.insertion-a-parameter-20"),
    ParameterId::new("part.insertion-a-parameter-21"),
    ParameterId::new("part.insertion-a-parameter-22"),
    ParameterId::new("part.insertion-a-parameter-23"),
    ParameterId::new("part.insertion-a-parameter-24"),
];

pub(super) const INSERTION_A_SLOT_NAMES: [&str; SLOTS] = [
    "Insertion A Parameter 1",
    "Insertion A Parameter 2",
    "Insertion A Parameter 3",
    "Insertion A Parameter 4",
    "Insertion A Parameter 5",
    "Insertion A Parameter 6",
    "Insertion A Parameter 7",
    "Insertion A Parameter 8",
    "Insertion A Parameter 9",
    "Insertion A Parameter 10",
    "Insertion A Parameter 11",
    "Insertion A Parameter 12",
    "Insertion A Parameter 13",
    "Insertion A Parameter 14",
    "Insertion A Parameter 15",
    "Insertion A Parameter 16",
    "Insertion A Parameter 17",
    "Insertion A Parameter 18",
    "Insertion A Parameter 19",
    "Insertion A Parameter 20",
    "Insertion A Parameter 21",
    "Insertion A Parameter 22",
    "Insertion A Parameter 23",
    "Insertion A Parameter 24",
];

pub(super) const INSERTION_B_IDS: [ParameterId; SLOTS] = [
    ParameterId::new("part.insertion-b-parameter-1"),
    ParameterId::new("part.insertion-b-parameter-2"),
    ParameterId::new("part.insertion-b-parameter-3"),
    ParameterId::new("part.insertion-b-parameter-4"),
    ParameterId::new("part.insertion-b-parameter-5"),
    ParameterId::new("part.insertion-b-parameter-6"),
    ParameterId::new("part.insertion-b-parameter-7"),
    ParameterId::new("part.insertion-b-parameter-8"),
    ParameterId::new("part.insertion-b-parameter-9"),
    ParameterId::new("part.insertion-b-parameter-10"),
    ParameterId::new("part.insertion-b-parameter-11"),
    ParameterId::new("part.insertion-b-parameter-12"),
    ParameterId::new("part.insertion-b-parameter-13"),
    ParameterId::new("part.insertion-b-parameter-14"),
    ParameterId::new("part.insertion-b-parameter-15"),
    ParameterId::new("part.insertion-b-parameter-16"),
    ParameterId::new("part.insertion-b-parameter-17"),
    ParameterId::new("part.insertion-b-parameter-18"),
    ParameterId::new("part.insertion-b-parameter-19"),
    ParameterId::new("part.insertion-b-parameter-20"),
    ParameterId::new("part.insertion-b-parameter-21"),
    ParameterId::new("part.insertion-b-parameter-22"),
    ParameterId::new("part.insertion-b-parameter-23"),
    ParameterId::new("part.insertion-b-parameter-24"),
];

pub(super) const INSERTION_B_SLOT_NAMES: [&str; SLOTS] = [
    "Insertion B Parameter 1",
    "Insertion B Parameter 2",
    "Insertion B Parameter 3",
    "Insertion B Parameter 4",
    "Insertion B Parameter 5",
    "Insertion B Parameter 6",
    "Insertion B Parameter 7",
    "Insertion B Parameter 8",
    "Insertion B Parameter 9",
    "Insertion B Parameter 10",
    "Insertion B Parameter 11",
    "Insertion B Parameter 12",
    "Insertion B Parameter 13",
    "Insertion B Parameter 14",
    "Insertion B Parameter 15",
    "Insertion B Parameter 16",
    "Insertion B Parameter 17",
    "Insertion B Parameter 18",
    "Insertion B Parameter 19",
    "Insertion B Parameter 20",
    "Insertion B Parameter 21",
    "Insertion B Parameter 22",
    "Insertion B Parameter 23",
    "Insertion B Parameter 24",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_every_documented_slot() {
        let slots = parsed();
        assert_eq!(slots.len(), TABLE.lines().count());
        let chorus = slot(List::Variation, 0x0180, 1).unwrap();
        assert_eq!(chorus.name, "LFO Speed");
        assert_eq!((chorus.min, chorus.max), (0, 127));
        assert_eq!(chorus.range, "0.0Hz – 39.70Hz");
        assert_eq!(chorus.page, 148);
        assert!(slot(List::Variation, 0x0180, 2).is_none());
        let reverb = slot(List::Reverb, 0x0080, 1).unwrap();
        assert_eq!(reverb.name, "Reverb Time");
        assert!(slot(List::Variation, 0x0C38, 1).is_none());
        assert!(
            slots
                .values()
                .all(|slot| slot.min <= slot.max && slot.max <= 0x3FFF)
        );
    }

    #[test]
    fn rejects_malformed_lines() {
        assert!(parse("xyz\t0180\t1\tName\t0\t1\tRange\t1").is_none());
        assert!(parse("var\t0180").is_none());
    }
}
