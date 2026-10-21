//! The preset Performances of the Performance List (Data List pages 2–17) and
//! how to load one into a Part.
//!
//! The MIDI Data Format (page 187) lists the banks of Single Part Performances
//! as Bank Select MSB 63 with LSB 0–31 for Preset 1–32, each holding 128
//! Programs, and the Bulk Control table (page 193) splits the presets into the
//! same 128-Performance banks (PRE 1–32). Performance number `n` of the
//! Performance List is therefore LSB `(n - 1) / 128` and Program `(n - 1) % 128`.
//!
//! Sending the Single Part bank and Program on a Part's MIDI channel (Part 1 on
//! channel 1 … Part 16 on channel 16, with MIDI I/O Mode set to Multi) places
//! the first Part of that Performance in the Part. The Data List does not
//! describe this; it is how the instrument behaves as described by Yamaha's
//! community forum, so the app reads the Part back to confirm it.

use std::sync::OnceLock;

use crate::domain::ListedValue;
use crate::protocol::ChannelMessage;

pub const COUNT: usize = 2227;
pub const SINGLE_PART_BANK_MSB: u8 = 63;
const BANK_SELECT_MSB: u8 = 0x00;
const BANK_SELECT_LSB: u8 = 0x20;
const PROGRAMS_PER_BANK: usize = 128;

const TABLE: &str = include_str!("performances.tsv");

struct Entry {
    name: &'static str,
    group: String,
}

static PERFORMANCES: OnceLock<Vec<ListedValue>> = OnceLock::new();

/// Every preset Performance, in Performance List order, grouped as
/// "Main Category/Sub Category".
pub fn all() -> &'static [ListedValue] {
    PERFORMANCES.get_or_init(|| {
        TABLE
            .lines()
            .filter_map(parse)
            .map(|entry| ListedValue {
                name: entry.name,
                group: Box::leak(entry.group.into_boxed_str()),
            })
            .collect()
    })
}

fn parse(line: &'static str) -> Option<Entry> {
    let mut columns = line.split('\t');
    let _number = columns.next()?;
    let name = columns.next()?;
    let main = columns.next()?;
    let sub = columns.next()?;
    Some(Entry {
        name,
        group: format!("{main}/{sub}"),
    })
}

/// Bank Select LSB and Program Change for Performance `number` (1-based).
pub fn bank_and_program(number: i32) -> Option<(u8, u8)> {
    let index = usize::try_from(number).ok()?.checked_sub(1)?;
    if index >= COUNT {
        return None;
    }
    let lsb = u8::try_from(index / PROGRAMS_PER_BANK).ok()?;
    let program = u8::try_from(index % PROGRAMS_PER_BANK).ok()?;
    Some((lsb, program))
}

/// Bank Select MSB, Bank Select LSB and Program Change that load Performance
/// `number` into `part` (1–16).
pub fn load(part: u8, number: i32) -> Option<Vec<ChannelMessage>> {
    let channel = part.checked_sub(1).filter(|channel| *channel <= 0x0F)?;
    let (lsb, program) = bank_and_program(number)?;
    Some(vec![
        ChannelMessage::control_change(channel, BANK_SELECT_MSB, SINGLE_PART_BANK_MSB)?,
        ChannelMessage::control_change(channel, BANK_SELECT_LSB, lsb)?,
        ChannelMessage::program_change(channel, program)?,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_every_preset_performance_in_order() {
        let performances = all();
        assert_eq!(performances.len(), COUNT);
        for (index, line) in TABLE.lines().enumerate() {
            let number: usize = line.split('\t').next().unwrap().parse().unwrap();
            assert_eq!(number, index + 1);
            assert_eq!(line.split('\t').count(), 4, "{line}");
        }
        assert_eq!(performances[0].name, "CFX + FM EP");
        assert_eq!(performances[2].name, "CFX Concert");
        assert_eq!(performances[2].group, "Piano/Acoustic");
        assert_eq!(performances[884].name, "1972");
        assert_eq!(performances[909].name, "1967 Keys");
        assert_eq!(performances[954].group, "Chromatic Perc/Bell");
        assert_eq!(performances[COUNT - 1].name, "Drone Generator");
    }

    #[test]
    fn numbers_map_to_banks_of_128() {
        assert_eq!(bank_and_program(1), Some((0, 0)));
        assert_eq!(bank_and_program(128), Some((0, 127)));
        assert_eq!(bank_and_program(129), Some((1, 0)));
        assert_eq!(bank_and_program(2227), Some((17, 50)));
        assert_eq!(bank_and_program(0), None);
        assert_eq!(bank_and_program(2228), None);
        assert_eq!(bank_and_program(-3), None);
    }

    #[test]
    fn loading_uses_the_part_channel() {
        let hex = |part, number| {
            load(part, number).map(|messages| {
                messages
                    .iter()
                    .map(ChannelMessage::to_hex)
                    .collect::<Vec<_>>()
                    .join(" | ")
            })
        };
        assert_eq!(hex(1, 3).as_deref(), Some("B0 00 3F | B0 20 00 | C0 02"));
        assert_eq!(hex(7, 955).as_deref(), Some("B6 00 3F | B6 20 07 | C6 3A"));
        assert_eq!(hex(16, 129).as_deref(), Some("BF 00 3F | BF 20 01 | CF 00"));
        assert_eq!(hex(0, 3), None);
        assert_eq!(hex(17, 3), None);
        assert_eq!(hex(2, 0), None);
    }
}
