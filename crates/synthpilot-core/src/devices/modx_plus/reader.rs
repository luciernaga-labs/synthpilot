use super::catalog::{self, Entry, Span};
use super::{MODEL_ID, PART_COUNT, awm2, drum, effects, fm_x, part};
use crate::domain::{EngineId, Layout, ParameterChange, PartData, Received};
use crate::protocol::SysExMessage;
use crate::protocol::yamaha::{self, Address, DeviceNumber, Incoming};

const BULK_HEADER: u8 = 0x0E;
const BULK_FOOTER: u8 = 0x0F;
const PART_EDIT_BUFFER: u8 = 0x52;
const COMMON_EDIT_BUFFER: u8 = 0x53;
const PERFORMANCE: u8 = 0x30;
const PERFORMANCE_NAME: u8 = 0x40;
const REVERB: u8 = 0x41;
const VARIATION: u8 = 0x42;
const INSERTION_A: u8 = 0x20;
const INSERTION_B: u8 = 0x30;
const FM_COMMON: u8 = 0x48;
const FM_OPERATOR: u8 = 0x49;
const NORMAL_ELEMENT_GROUP_1: u8 = 0x41;
const NORMAL_ELEMENT_GROUP_2: u8 = 0x42;

struct Catalog {
    engine: EngineId,
    layout: Layout,
    entries: Vec<Entry>,
}

pub(super) struct Decoder {
    catalogs: Vec<Catalog>,
}

impl Decoder {
    pub(super) fn new() -> Self {
        let layout = |elements| Layout {
            parts: PART_COUNT,
            elements,
        };
        Self {
            catalogs: vec![
                Catalog {
                    engine: awm2::ENGINE_ID,
                    layout: layout(awm2::ELEMENT_COUNT),
                    entries: awm2::entries(),
                },
                Catalog {
                    engine: fm_x::ENGINE_ID,
                    layout: layout(fm_x::OPERATOR_COUNT),
                    entries: fm_x::entries(),
                },
                Catalog {
                    engine: drum::ENGINE_ID,
                    layout: layout(drum::KEY_COUNT),
                    entries: drum::entries(),
                },
                Catalog {
                    engine: effects::ENGINE_ID,
                    layout: layout(0),
                    entries: effects::entries(),
                },
            ],
        }
    }

    pub(super) fn requests(device_number: DeviceNumber) -> Vec<SysExMessage> {
        let common = Address::new(BULK_HEADER, COMMON_EDIT_BUFFER, 0x00).ok();
        (0..PART_COUNT)
            .filter_map(|part| Address::new(BULK_HEADER, PART_EDIT_BUFFER, part).ok())
            .chain(common)
            .filter_map(|address| yamaha::dump_request(device_number, MODEL_ID, address).ok())
            .collect()
    }

    pub(super) fn decode(&self, bytes: &[u8]) -> Received {
        match yamaha::parse(bytes, MODEL_ID) {
            Ok(Incoming::BulkDump { address, data, .. }) => self.bulk(address, &data),
            Ok(Incoming::ParameterChange { address, data, .. }) => self.change(address, &data),
            Err(_) => Received::Ignored,
        }
    }

    fn bulk(&self, address: Address, data: &[u8]) -> Received {
        let (high, mid, low) = (address.high(), address.mid(), address.low());
        match high {
            BULK_HEADER if mid == PART_EDIT_BUFFER && low < PART_COUNT => {
                Received::PartStarted(low + 1)
            }
            BULK_FOOTER if mid == PART_EDIT_BUFFER && low < PART_COUNT => {
                Received::PartFinished(low + 1)
            }
            BULK_FOOTER if mid == COMMON_EDIT_BUFFER && low == 0 => Received::CommonFinished,
            PERFORMANCE if mid == PERFORMANCE_NAME && low == 0 => {
                part::name(data).map_or(Received::Ignored, Received::SoundName)
            }
            PERFORMANCE if (mid == REVERB || mid == VARIATION) && low == 0 => {
                Received::Common(self.values(effects::ENGINE_ID, address, data, Span::Block))
            }
            part::HIGH if mid < PART_COUNT && low == 0 => Received::Part(PartData {
                part: mid + 1,
                engine: None,
                name: part::name(data),
                values: self.values(awm2::ENGINE_ID, address, data, Span::Block),
            }),
            part::HIGH if (mid & 0xF0 == INSERTION_A || mid & 0xF0 == INSERTION_B) && low == 0 => {
                Received::Part(PartData {
                    part: (mid & 0x0F) + 1,
                    engine: None,
                    name: None,
                    values: self.values(awm2::ENGINE_ID, address, data, Span::Block),
                })
            }
            NORMAL_ELEMENT_GROUP_1 | NORMAL_ELEMENT_GROUP_2 => Received::Part(PartData {
                part: (mid & 0x0F) + 1,
                engine: Some(awm2::ENGINE_ID),
                name: None,
                values: self.values(awm2::ENGINE_ID, address, data, Span::Block),
            }),
            FM_COMMON | FM_OPERATOR => {
                let part = if high == FM_COMMON { mid } else { mid & 0x0F };
                Received::Part(PartData {
                    part: part + 1,
                    engine: Some(fm_x::ENGINE_ID),
                    name: None,
                    values: self.values(fm_x::ENGINE_ID, address, data, Span::Block),
                })
            }
            drum::FIRST_HIGH..=drum::LAST_HIGH => Received::Part(PartData {
                part: (high & 0x0F) + 1,
                engine: Some(drum::ENGINE_ID),
                name: None,
                values: Vec::new(),
            }),
            _ => Received::Ignored,
        }
    }

    fn change(&self, address: Address, data: &[u8]) -> Received {
        self.catalogs
            .iter()
            .map(|catalog| {
                catalog::decode(
                    &catalog.entries,
                    catalog.layout,
                    address,
                    data,
                    Span::Single,
                )
            })
            .find(|values| !values.is_empty())
            .and_then(|values| {
                let location = values.first()?.location;
                if location.is_performance() {
                    return Some(Received::Common(values));
                }
                let part = location.part_number();
                Some(Received::Part(PartData {
                    part,
                    engine: None,
                    name: None,
                    values,
                }))
            })
            .unwrap_or(Received::Ignored)
    }

    fn values(
        &self,
        engine: EngineId,
        address: Address,
        data: &[u8],
        span: Span,
    ) -> Vec<ParameterChange> {
        self.catalogs
            .iter()
            .find(|catalog| catalog.engine == engine)
            .map(|catalog| catalog::decode(&catalog.entries, catalog.layout, address, data, span))
            .unwrap_or_default()
    }
}
