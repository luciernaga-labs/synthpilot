use std::sync::OnceLock;

use crate::domain::ListedValue;

pub const COUNT: usize = 6347;
pub(super) const COUNT_I32: i32 = 6347;
pub(super) const COUNT_U32: u32 = 6347;

const TABLE: &str = include_str!("waves.tsv");

static WAVES: OnceLock<Vec<ListedValue>> = OnceLock::new();

pub fn all() -> &'static [ListedValue] {
    WAVES.get_or_init(|| TABLE.lines().filter_map(parse).collect())
}

fn parse(line: &'static str) -> Option<ListedValue> {
    let mut columns = line.split('\t');
    let _number = columns.next()?;
    let name = columns.next()?;
    let group = columns.next()?;
    Some(ListedValue { name, group })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_every_preset_wave_in_order() {
        let waves = all();
        assert_eq!(waves.len(), COUNT);
        assert_eq!(usize::try_from(COUNT_I32).ok(), Some(COUNT));
        assert_eq!(usize::try_from(COUNT_U32).ok(), Some(COUNT));
        for (index, line) in TABLE.lines().enumerate() {
            let number: usize = line.split('\t').next().unwrap().parse().unwrap();
            assert_eq!(number, index + 1);
        }
        assert_eq!(waves[0].name, "CF3 Stretch Sw St");
        assert_eq!(waves[165].name, "EP1 Sw");
        assert_eq!(waves[165].group, "Keys/Rd");
        assert_eq!(waves[263].name, "DX1");
        assert_eq!(waves[COUNT - 1].name, "Reverse Cymbal2");
        assert!(waves.iter().all(|wave| wave.group.contains('/')));
    }

    #[test]
    fn incomplete_lines_are_skipped() {
        assert_eq!(parse("1\tName"), None);
        assert_eq!(parse(""), None);
    }
}
