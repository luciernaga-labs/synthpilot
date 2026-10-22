use std::collections::BTreeMap;
use std::fmt::Write as _;

use synthpilot_core::domain::{
    EngineProfile, ListedValue, ParameterDefinition, Part, Scope, ValueFormat,
};

#[derive(Clone, Copy)]
pub struct PartContext<'a> {
    pub number: u8,
    pub engine: &'a dyn EngineProfile,
    pub part: &'a Part,
}

#[derive(Clone)]
pub struct Context<'a> {
    pub device: String,
    pub sound: Option<&'a str>,
    pub performance: Option<PartContext<'a>>,
    /// Every Part of the sound that has an engine, in Part order.
    pub parts: Vec<PartContext<'a>>,
    /// The Part open in the editor, shown to the model in full.
    pub selected: Option<u8>,
    /// Preset sounds that can be loaded into a Part, numbered from 1.
    pub presets: &'a [ListedValue],
    /// How many Parts the instrument has, used or empty.
    pub part_count: usize,
}

impl<'a> Context<'a> {
    pub fn part(&self, number: u8) -> Option<PartContext<'a>> {
        self.parts
            .iter()
            .copied()
            .find(|part| part.number == number)
    }

    pub fn selected_part(&self) -> Option<PartContext<'a>> {
        self.selected.and_then(|number| self.part(number))
    }

    pub(crate) fn targets(&self) -> impl Iterator<Item = PartContext<'a>> + '_ {
        self.performance
            .into_iter()
            .chain(self.parts.iter().copied())
    }

    pub fn describe(&self) -> String {
        let mut text = format!("Device: {}\n", self.device);
        if let Some(name) = self.sound {
            let _ = writeln!(text, "Performance name: {name}");
        }
        if !self.parts.is_empty() {
            let numbers: Vec<String> = self
                .parts
                .iter()
                .map(|part| part.number.to_string())
                .collect();
            let _ = writeln!(
                text,
                "Parts in this Performance: {}. Every Part sounds together when you play, unless its Note Limit or Velocity Limit leaves it out.",
                numbers.join(", ")
            );
        }
        if let Some(performance) = self.performance {
            text.push_str(
                "\n## Performance effects\nShared by every Part. Give \"part\": null and \"element\": null.\n",
            );
            section(&mut text, performance);
        }
        if let Some(part) = self.selected_part() {
            heading(&mut text, part, "## ", " (open in the editor)");
            section(&mut text, part);
        }
        let others: Vec<PartContext<'_>> = self
            .parts
            .iter()
            .copied()
            .filter(|part| Some(part.number) != self.selected)
            .collect();
        if !self.presets.is_empty() {
            let _ = writeln!(
                text,
                "\n## Sounds to load into a Part\n{} preset sounds can replace a Part's sound or fill an empty Part (Parts not listed above are empty). Look up \"Sounds: <group>\" to see a group's numbered sounds, then add {{\"part\": <number>, \"sound\": <number>, \"reason\": \"...\"}} to \"load\". Loading replaces every setting of that Part, so don't change that Part in the same answer. Only Parts 1 to 8 play from the keys. Groups: {}",
                self.presets.len(),
                groups(self.presets)
            );
        }
        if !others.is_empty() {
            text.push_str(
                "\n## Other Parts\nTheir Part parameters use the same ids as above and can be changed directly with their \"part\" number. To see a Part's full catalog, including its per-element or per-operator values, look up \"Part <number>\".\n",
            );
            for part in others {
                heading(&mut text, part, "### ", "");
                summary(&mut text, part);
            }
        }
        text
    }

    pub fn listings(&self) -> BTreeMap<String, String> {
        let mut listings: BTreeMap<String, String> = BTreeMap::new();
        let mut seen: Vec<*const ListedValue> = Vec::new();
        let mut engines: Vec<&str> = Vec::new();
        for part in &self.parts {
            if Some(part.number) != self.selected {
                let mut detail = String::new();
                heading(&mut detail, *part, "## ", "");
                section(&mut detail, *part);
                listings.insert(format!("Part {}", part.number), detail);
            }
        }
        for (number, preset) in (1..).zip(self.presets) {
            let entry = listings
                .entry(format!("Sounds: {}", preset.group))
                .or_default();
            if !entry.is_empty() {
                entry.push_str("; ");
            }
            let _ = write!(entry, "{number}={}", preset.name);
        }
        for target in self.targets() {
            let engine = target.engine.id().as_str();
            if engines.contains(&engine) {
                continue;
            }
            engines.push(engine);
            for parameter in target.engine.parameters() {
                for value in parameter.range.min()..=parameter.range.max() {
                    if !matches!(parameter.format, ValueFormat::Choice(_)) {
                        break;
                    }
                    let dependents = target.engine.dependents(parameter, value);
                    if dependents.is_empty() {
                        continue;
                    }
                    let key = format!("{}: {}", parameter.name, parameter.display_value(value));
                    let lines: Vec<String> = dependents
                        .iter()
                        .map(|dependent| {
                            format!(
                                "{} = {} {}..{} ({})",
                                dependent.id,
                                dependent.name,
                                dependent.range.min(),
                                dependent.range.max(),
                                dependent.help.unwrap_or_default()
                            )
                        })
                        .collect();
                    listings.insert(key, lines.join("; "));
                }
                let ValueFormat::Listed { first, values } = parameter.format else {
                    continue;
                };
                if seen.contains(&values.as_ptr()) {
                    continue;
                }
                seen.push(values.as_ptr());
                for (number, listed) in (first..).zip(values) {
                    let entry = listings.entry(listed.group.to_owned()).or_default();
                    if !entry.is_empty() {
                        entry.push_str("; ");
                    }
                    let _ = write!(entry, "{number}={}", listed.name);
                }
            }
        }
        listings
    }
}

fn heading(text: &mut String, part: PartContext<'_>, level: &str, note: &str) {
    let layout = part.engine.layout();
    let _ = write!(
        text,
        "\n{level}Part {} · {}",
        part.number,
        part.engine.name()
    );
    if let Some(name) = part.part.name() {
        let _ = write!(text, " · \"{name}\"");
    }
    let _ = writeln!(text, "{note}\n{}", part.engine.description());
    if layout.elements > 0 {
        let _ = writeln!(
            text,
            "Give \"part\": {}. Parameters marked \"per {}\" need \"element\" from 1 to {}. Others need \"element\" null.",
            part.number,
            part.engine.element_label().to_lowercase(),
            layout.elements
        );
    } else {
        let _ = writeln!(
            text,
            "Give \"part\": {} and \"element\": null.",
            part.number
        );
    }
}

/// One line with the current value of every editable Part parameter.
fn summary(text: &mut String, context: PartContext<'_>) {
    let values: Vec<String> = context
        .engine
        .parameters()
        .iter()
        .filter(|parameter| parameter.is_implemented() && parameter.scope == Scope::Part)
        .filter_map(|parameter| context.engine.refine(parameter, context.part))
        .filter_map(|parameter| {
            let value = context.part.value(parameter.id, None)?;
            Some(format!("{}={}", parameter.id, shown(&parameter, value)))
        })
        .collect();
    let _ = writeln!(text, "now: {}", values.join(", "));
}

fn shown(parameter: &ParameterDefinition, value: i32) -> String {
    match parameter.format {
        ValueFormat::Number if parameter.unit.is_none() => value.to_string(),
        _ => format!("{value} ({})", parameter.display_value(value)),
    }
}

fn groups(values: &[ListedValue]) -> String {
    let mut groups: Vec<(&str, usize)> = Vec::new();
    for value in values {
        match groups.iter_mut().find(|(group, _)| *group == value.group) {
            Some((_, count)) => *count += 1,
            None => groups.push((value.group, 1)),
        }
    }
    groups
        .iter()
        .map(|(group, count)| format!("{group} ({count})"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn section(text: &mut String, context: PartContext<'_>) {
    let elements = context.engine.layout().elements;
    let mut missing = Vec::new();
    for parameter in context.engine.parameters() {
        if !parameter.is_implemented() {
            missing.push(parameter.name);
            continue;
        }
        if parameter.scope == Scope::Element && elements > 20 {
            continue;
        }
        let Some(parameter) = context.engine.refine(parameter, context.part) else {
            continue;
        };
        line(text, context, &parameter, elements);
    }
    if !missing.is_empty() {
        let _ = writeln!(text, "Not editable yet: {}.", missing.join(", "));
    }
}

fn line(
    text: &mut String,
    context: PartContext<'_>,
    parameter: &ParameterDefinition,
    elements: u8,
) {
    let _ = write!(
        text,
        "- {} | {} | {} | {}..{}",
        parameter.id,
        parameter.name,
        parameter.category,
        parameter.range.min(),
        parameter.range.max()
    );
    match parameter.format {
        ValueFormat::Choice(_) => {
            let options: Vec<String> = (parameter.range.min()..=parameter.range.max())
                .map(|value| format!("{value}={}", parameter.display_value(value)))
                .collect();
            let _ = write!(text, " | options: {}", options.join(", "));
            if context.engine.reshapes(parameter.id) {
                let _ = write!(
                    text,
                    " | each option has its own parameters; ask with \"lookup\" for \"{}: <option name>\" to see them",
                    parameter.name
                );
            }
        }
        ValueFormat::Listed { values, .. } => {
            let _ = write!(
                text,
                " | numbered list in groups, ask with \"lookup\" to see a group: {}",
                groups(values)
            );
        }
        ValueFormat::Number if parameter.unit.is_none() => {}
        _ => {
            let _ = write!(text, " ({})", parameter.display_range());
        }
    }
    if let Some(help) = parameter.help {
        let _ = write!(text, " | {help}");
    }
    let shown = |value: i32| shown(parameter, value);
    match parameter.scope {
        Scope::Element => {
            let label = context.engine.element_label().to_lowercase();
            let values: Vec<String> = (1..=elements)
                .map(|element| {
                    let value = context
                        .part
                        .value(parameter.id, Some(element))
                        .map_or_else(|| "?".to_owned(), shown);
                    format!("{element}: {value}")
                })
                .collect();
            let _ = writeln!(text, " | per {label} | now {}", values.join("; "));
        }
        Scope::Performance | Scope::Part => {
            let value = context
                .part
                .value(parameter.id, None)
                .map_or_else(|| "unknown".to_owned(), shown);
            let _ = writeln!(text, " | now {value}");
        }
    }
}

#[cfg(test)]
mod tests {
    use synthpilot_core::devices::modx_plus::{ModxPlus, awm2, part};
    use synthpilot_core::domain::{DeviceProfile, Location, ParameterChange, Sound};

    use super::*;

    #[test]
    fn describes_the_effects_and_the_selected_part() {
        let device = ModxPlus::new();
        let engine = device.engine(awm2::ENGINE_ID).unwrap();
        let mut sound = Sound::new(&device);
        sound
            .load(
                1,
                engine,
                Some("Init Normal".to_owned()),
                &[ParameterChange::new(part::VOLUME, Location::part(1), 90)],
            )
            .unwrap();
        let effects = device.common().unwrap();
        let context = Context {
            device: "Yamaha MODX+".to_owned(),
            sound: Some("Init Normal"),
            performance: Some(PartContext {
                number: 0,
                engine: effects,
                part: sound.common().unwrap(),
            }),
            parts: vec![PartContext {
                number: 1,
                engine,
                part: sound.part(1).unwrap(),
            }],
            selected: Some(1),
            presets: &[],
            part_count: 16,
        };
        let text = context.describe();
        assert!(text.starts_with("Device: Yamaha MODX+\nPerformance name: Init Normal\n"));
        assert!(text.contains("## Performance effects"));
        assert!(text.contains(
            "- effects.reverb-type | Reverb Type | Reverb | 0..12 | options: 0=NO EFFECT, 1=HD HALL"
        ));
        assert!(text.contains("| now 2 (REV-X HALL)"));
        assert!(text.contains("## Part 1 · AWM2 · \"Init Normal\""));
        assert!(text.contains("need \"element\" from 1 to 8"));
        assert!(text.contains("- part.volume | Volume |"));
        assert!(text.contains("| now 90\n"));
        assert!(text.contains("- part.pan | Pan | General | -63..63 (L63 … R63)"));
        assert!(text.contains("- awm2.element-switch |"));
        assert!(text.contains("| per element | now 1: 1 (On); 2: 0 (Off)"));
        assert!(text.contains("- part.eq-low-gain |"));
        assert!(text.contains("(-12.00 dB … +12.00 dB)"));
        assert!(text.contains("Not editable yet: "));
        assert!(text.contains(
            "- awm2.wave-number | Wave | Oscillator | 1..6347 | numbered list in groups"
        ));
        assert!(text.contains("Keys/Rd (10)") || text.contains("Keys/Rd ("));
        assert!(text.contains("now 1: 1 (1 CF3 Stretch Sw St)"));
        assert!(text.contains("Filter Type"));

        let listings = context.listings();
        assert!(listings["Keys/Rd"].starts_with("166=EP1 Sw; 167=EP1 Soft"));
        assert!(listings["Keys/FM"].contains("264=DX1"));
        assert!(listings["Variation Type: SPX CHORUS"].starts_with(
            "effects.variation-parameter-1 = LFO Speed 0..127 (0.0Hz – 39.70Hz); effects.variation-parameter-2 = LFO Depth"
        ));
        assert!(listings["Insertion A Type: ROTARY SPEAKER 1"].contains("Rotor Speed Slow"));
        assert!(listings["Reverb Type: HD PLATE"].contains("Plate Type"));
        assert!(!listings.contains_key("Variation Type: NO EFFECT"));
        assert!(text.contains("ask with \"lookup\" for \"Variation Type: <option name>\""));
        assert!(text.contains(
            "- effects.variation-parameter-1 | LFO Speed | Variation | 0..127 | 0.0Hz – 39.70Hz"
        ));
        assert!(!text.contains("effects.variation-parameter-2 |"));
        assert!(!text.contains("part.insertion-a-parameter-1 |"));
        let waves: usize = listings
            .iter()
            .filter(|(group, _)| !group.contains(": "))
            .map(|(_, list)| list.matches(';').count() + 1)
            .sum();
        assert_eq!(waves, 6347);
    }

    #[test]
    fn drum_keys_are_left_out() {
        let device = ModxPlus::new();
        let engine = device
            .engine(synthpilot_core::devices::modx_plus::drum::ENGINE_ID)
            .unwrap();
        let mut sound = Sound::new(&device);
        sound.load(2, engine, None, &[]).unwrap();
        let context = Context {
            device: "Yamaha MODX+".to_owned(),
            sound: None,
            performance: None,
            parts: vec![PartContext {
                number: 2,
                engine,
                part: sound.part(2).unwrap(),
            }],
            selected: Some(2),
            presets: &[],
            part_count: 16,
        };
        let text = context.describe();
        assert!(!text.contains("Performance name"));
        assert!(!text.contains("## Performance effects"));
        assert!(text.contains("## Part 2 · Drum (open in the editor)\n"));
        assert!(!text.contains("| per key |"));
        assert!(text.contains("| now unknown") || text.contains("| now "));
    }

    #[test]
    fn every_part_is_offered_and_others_are_summarized() {
        let device = ModxPlus::new();
        let awm2_engine = device.engine(awm2::ENGINE_ID).unwrap();
        let fm_engine = device
            .engine(synthpilot_core::devices::modx_plus::fm_x::ENGINE_ID)
            .unwrap();
        let mut sound = Sound::new(&device);
        sound
            .load(1, awm2_engine, Some("CFX + FM EP".to_owned()), &[])
            .unwrap();
        sound
            .load(
                5,
                fm_engine,
                Some("KEYBOARD".to_owned()),
                &[ParameterChange::new(part::VOLUME, Location::part(5), 80)],
            )
            .unwrap();
        let parts = [1, 5]
            .into_iter()
            .map(|number| {
                let values = sound.part(number).unwrap();
                PartContext {
                    number,
                    engine: device.engine(values.engine()).unwrap(),
                    part: values,
                }
            })
            .collect();
        let context = Context {
            device: "Yamaha MODX+".to_owned(),
            sound: None,
            performance: None,
            parts,
            selected: Some(1),
            presets: &[],
            part_count: 16,
        };
        let text = context.describe();
        assert!(text.contains("Parts in this Performance: 1, 5."));
        assert!(text.contains("## Part 1 · AWM2 · \"CFX + FM EP\" (open in the editor)"));
        assert!(text.contains("Give \"part\": 1."));
        assert!(text.contains("## Other Parts"));
        assert!(text.contains("### Part 5 · FM-X · \"KEYBOARD\""));
        assert!(text.contains("part.volume=80"));
        assert!(
            !text.contains("- fm-x."),
            "the other Part is only summarized"
        );
        let listings = context.listings();
        assert!(!listings.contains_key("Part 1"));
        let detail = &listings["Part 5"];
        assert!(detail.contains("Give \"part\": 5."));
        assert!(detail.contains("| per operator | now 1:"));
        assert_eq!(context.part(5).map(|part| part.number), Some(5));
        assert!(!text.contains("## Sounds to load"));
        let with_sounds = Context {
            presets: synthpilot_core::devices::modx_plus::performances::all(),
            ..context.clone()
        };
        let text = with_sounds.describe();
        assert!(text.contains("## Sounds to load into a Part\n2227 preset sounds"));
        assert!(text.contains("Chromatic Perc/Bell ("));
        let listings = with_sounds.listings();
        assert!(listings["Sounds: Chromatic Perc/Bell"].contains("955=Gamelan"));
        assert!(
            listings["Sounds: Piano/Acoustic"]
                .starts_with("1=CFX + FM EP; 2=CFX + FM EP 2; 3=CFX Concert")
        );
        assert!(context.part(2).is_none());
    }
}
