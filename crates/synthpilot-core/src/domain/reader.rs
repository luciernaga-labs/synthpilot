use std::collections::{BTreeMap, BTreeSet};

use super::{DeviceProfile, EngineId, ParameterChange, Sound};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartData {
    pub part: u8,
    pub engine: Option<EngineId>,
    pub name: Option<String>,
    pub values: Vec<ParameterChange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Received {
    PartStarted(u8),
    PartFinished(u8),
    Part(PartData),
    SoundName(String),
    CommonFinished,
    Ignored,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct PartRead {
    engine: Option<EngineId>,
    name: Option<String>,
    values: Vec<ParameterChange>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SoundReader {
    name: Option<String>,
    parts: BTreeMap<u8, PartRead>,
    finished: BTreeSet<u8>,
    common_finished: bool,
    accepted: usize,
}

impl SoundReader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn accept(&mut self, received: Received) -> bool {
        match received {
            Received::Ignored => return false,
            Received::PartStarted(part) => {
                self.parts.entry(part).or_default();
            }
            Received::PartFinished(part) => {
                self.finished.insert(part);
            }
            Received::SoundName(name) => self.name = Some(name),
            Received::CommonFinished => self.common_finished = true,
            Received::Part(data) => {
                let part = self.parts.entry(data.part).or_default();
                if data.engine.is_some() {
                    part.engine = data.engine;
                }
                if data.name.is_some() {
                    part.name = data.name;
                }
                part.values.extend(data.values);
            }
        }
        self.accepted += 1;
        true
    }

    pub const fn accepted(&self) -> usize {
        self.accepted
    }

    pub fn finished(&self) -> usize {
        self.finished.len() + usize::from(self.common_finished)
    }

    pub fn into_sound(self, device: &dyn DeviceProfile) -> Sound {
        let mut sound = Sound::new(device);
        sound.rename(self.name);
        for (number, part) in self.parts {
            let Some(engine) = part.engine.and_then(|engine| device.engine(engine)) else {
                continue;
            };
            let _ = sound.load(number, engine, part.name, &part.values);
        }
        sound
    }
}
