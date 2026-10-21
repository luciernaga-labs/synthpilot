use serde::Deserialize;
use serde_json::Value;
use synthpilot_core::domain::{Location, ParameterChange, ParameterDefinition, Part, Scope};

use super::context::{Context, PartContext};

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Proposal {
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub changes: Vec<Suggestion>,
    #[serde(default)]
    pub lookup: Vec<String>,
    #[serde(default)]
    pub load: Vec<LoadSuggestion>,
}

/// A preset sound the model wants in a Part.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LoadSuggestion {
    #[serde(default)]
    pub part: Option<u8>,
    pub sound: Value,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Suggestion {
    pub parameter: String,
    #[serde(default)]
    pub part: Option<u8>,
    #[serde(default)]
    pub element: Option<u8>,
    pub value: Value,
    #[serde(default)]
    pub reason: String,
}

impl Proposal {
    pub fn parse(text: &str) -> Result<Self, String> {
        let start = text.find('{');
        let end = text.rfind('}');
        let json = match (start, end) {
            (Some(start), Some(end)) if start < end => &text[start..=end],
            _ => return Err("the reply did not contain a JSON object".to_owned()),
        };
        serde_json::from_str(json).map_err(|error| error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub name: String,
    pub reason: String,
    pub change: Option<ParameterChange>,
    /// A preset sound to load: the Part and the preset number.
    pub load: Option<(u8, i32)>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub problem: Option<String>,
}

impl Resolved {
    pub fn is_applicable(&self) -> bool {
        (self.change.is_some() || self.load.is_some()) && self.problem.is_none()
    }

    fn rejected(name: String, reason: String, problem: String) -> Self {
        Self {
            name,
            reason,
            change: None,
            load: None,
            from: None,
            to: None,
            problem: Some(problem),
        }
    }
}

pub fn resolve(context: &Context<'_>, proposal: &Proposal) -> Vec<Resolved> {
    let mut planned: Vec<(u8, Part)> = context
        .targets()
        .map(|target| (target.number, target.part.clone()))
        .collect();
    for suggestion in &proposal.changes {
        let Ok((target, definition)) = find(context, suggestion) else {
            continue;
        };
        if !target.engine.reshapes(definition.id) {
            continue;
        }
        let Some(value) = value_of(definition, &suggestion.value) else {
            continue;
        };
        let location = Location::for_scope(definition.scope, target.number, 1);
        let change = ParameterChange::new(definition.id, location, value);
        if definition
            .validate(&change, target.engine.layout())
            .is_err()
        {
            continue;
        }
        if let Some((_, part)) = planned
            .iter_mut()
            .find(|(number, _)| *number == target.number)
        {
            *part = part.with(&change);
        }
    }
    let loads: Vec<Resolved> = proposal
        .load
        .iter()
        .map(|suggestion| resolve_load(context, suggestion))
        .collect();
    let loaded: Vec<u8> = loads
        .iter()
        .filter(|row| row.is_applicable())
        .filter_map(|row| row.load.map(|(part, _)| part))
        .collect();
    let changes = proposal.changes.iter().map(|suggestion| {
        let mut row = resolve_one(context, &planned, suggestion);
        let part = row.change.map(|change| change.location.part_number());
        if row.problem.is_none() && part.is_some_and(|part| loaded.contains(&part)) {
            row.problem = Some(
                "This Part gets a new sound first. Ask again after it loads to adjust it."
                    .to_owned(),
            );
        }
        row
    });
    loads.into_iter().chain(changes).collect()
}

fn resolve_load(context: &Context<'_>, suggestion: &LoadSuggestion) -> Resolved {
    let reason = suggestion.reason.clone();
    let Some(part) = suggestion.part.or(context.selected) else {
        return Resolved::rejected(
            "Sound".to_owned(),
            reason,
            "Choose which Part gets the sound.".to_owned(),
        );
    };
    let name = format!("Part {part} · Sound");
    if part == 0 || usize::from(part) > context.part_count {
        return Resolved::rejected(name, reason, format!("There is no Part {part}."));
    }
    let number = match &suggestion.sound {
        Value::Number(number) => number
            .as_i64()
            .and_then(|number| i32::try_from(number).ok()),
        Value::String(text) => (1..).zip(context.presets).find_map(|(number, preset)| {
            preset
                .name
                .eq_ignore_ascii_case(text.trim())
                .then_some(number)
        }),
        _ => None,
    };
    let preset = number.and_then(|number| {
        let index = usize::try_from(number.checked_sub(1)?).ok()?;
        context.presets.get(index).map(|preset| (number, preset))
    });
    let Some((number, preset)) = preset else {
        return Resolved::rejected(
            name,
            reason,
            format!("\"{}\" is not a sound on this keyboard.", suggestion.sound),
        );
    };
    let from = context.part(part).map(|current| {
        current
            .part
            .name()
            .map_or_else(|| "Unnamed sound".to_owned(), ToOwned::to_owned)
    });
    Resolved {
        name,
        reason,
        change: None,
        load: Some((part, number)),
        from: Some(from.unwrap_or_else(|| "Empty".to_owned())),
        to: Some(format!("{number} {} ({})", preset.name, preset.group)),
        problem: None,
    }
}

/// The Part (or the Performance) a suggestion edits and its parameter.
/// Performance parameters ignore "part"; Part and Element parameters go to the
/// Part the model names, or to the Part open in the editor.
fn find<'a>(
    context: &Context<'a>,
    suggestion: &Suggestion,
) -> Result<(PartContext<'a>, &'a ParameterDefinition), String> {
    let id = suggestion.parameter.as_str();
    let lookup = |target: PartContext<'a>| {
        target
            .engine
            .parameters()
            .iter()
            .find(|parameter| parameter.id.as_str() == id)
            .map(|parameter| (target, parameter))
    };
    if let Some(found) = context.performance.and_then(lookup) {
        return Ok(found);
    }
    let number = suggestion.part.or(context.selected);
    let Some(number) = number else {
        return Err("Choose which Part to change.".to_owned());
    };
    let Some(target) = context.part(number) else {
        return Err(format!("Part {number} isn't in this Performance."));
    };
    lookup(target).ok_or_else(|| "This parameter doesn't exist for this Part.".to_owned())
}

fn resolve_one(context: &Context<'_>, planned: &[(u8, Part)], suggestion: &Suggestion) -> Resolved {
    let reason = suggestion.reason.clone();
    let (target, generic) = match find(context, suggestion) {
        Ok(found) => found,
        Err(problem) => {
            let name = match suggestion.part {
                Some(number) => format!("Part {number} · {}", suggestion.parameter),
                None => suggestion.parameter.clone(),
            };
            return Resolved::rejected(name, reason, problem);
        }
    };
    let planned_part = planned
        .iter()
        .find(|(number, _)| *number == target.number)
        .map_or(target.part, |(_, part)| part);
    let Some(refined) = target.engine.refine(generic, planned_part) else {
        return Resolved::rejected(
            generic.name.to_owned(),
            reason,
            "This parameter isn't used by the chosen effect type.".to_owned(),
        );
    };
    let definition = &refined;
    let label = if definition.name == generic.name {
        definition.name.to_owned()
    } else {
        format!("{} · {}", definition.category, definition.name)
    };
    let name = match (definition.scope, suggestion.element) {
        (Scope::Element, Some(element)) => {
            format!("{label} · {} {element}", target.engine.element_label())
        }
        _ => label,
    };
    let name = if definition.scope != Scope::Performance && context.parts.len() > 1 {
        format!("Part {} · {name}", target.number)
    } else {
        name
    };
    if !definition.is_implemented() {
        return Resolved::rejected(
            name,
            reason,
            "SynthPilot can't edit this parameter yet.".to_owned(),
        );
    }
    let Some(value) = value_of(definition, &suggestion.value) else {
        return Resolved::rejected(
            name,
            reason,
            format!("\"{}\" is not a valid value.", suggestion.value),
        );
    };
    let location = match definition.scope {
        Scope::Performance => Location::performance(),
        Scope::Part => Location::part(target.number),
        Scope::Element => match suggestion.element {
            Some(element) => Location::element(target.number, element),
            None => {
                return Resolved::rejected(
                    name,
                    reason,
                    format!(
                        "Choose which {} to change.",
                        target.engine.element_label().to_lowercase()
                    ),
                );
            }
        },
    };
    let change = ParameterChange::new(definition.id, location, value);
    if let Err(error) = definition.validate(&change, target.engine.layout()) {
        return Resolved::rejected(name, reason, error.to_string());
    }
    let current = target.part.value(definition.id, location.element_number());
    Resolved {
        name,
        reason,
        change: Some(change),
        load: None,
        from: current.map(|value| definition.display_value(value)),
        to: Some(definition.display_value(value)),
        problem: (current == Some(value)).then(|| "Already set to this value.".to_owned()),
    }
}

fn value_of(definition: &ParameterDefinition, value: &Value) -> Option<i32> {
    match value {
        Value::Number(number) => number
            .as_i64()
            .or_else(|| number.as_f64().map(round))
            .and_then(|number| i32::try_from(number).ok()),
        Value::String(text) => definition
            .value_named(text)
            .or_else(|| text.trim().parse().ok()),
        _ => None,
    }
}

#[expect(clippy::cast_possible_truncation)]
fn round(number: f64) -> i64 {
    number.round() as i64
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use synthpilot_core::devices::modx_plus::{ModxPlus, awm2, effects, part};
    use synthpilot_core::domain::{DeviceProfile, Sound};

    use super::*;

    fn sound(device: &ModxPlus) -> Sound {
        let mut sound = Sound::new(device);
        let engine = device.engine(awm2::ENGINE_ID).unwrap();
        sound
            .load(
                3,
                engine,
                None,
                &[ParameterChange::new(
                    part::REVERB_SEND,
                    Location::part(3),
                    12,
                )],
            )
            .unwrap();
        sound
    }

    fn context<'a>(device: &'a ModxPlus, sound: &'a Sound) -> Context<'a> {
        Context {
            device: "MODX+".to_owned(),
            sound: None,
            performance: Some(PartContext {
                number: 0,
                engine: device.common().unwrap(),
                part: sound.common().unwrap(),
            }),
            parts: vec![PartContext {
                number: 3,
                engine: device.engine(awm2::ENGINE_ID).unwrap(),
                part: sound.part(3).unwrap(),
            }],
            selected: Some(3),
            presets: synthpilot_core::devices::modx_plus::performances::all(),
            part_count: 16,
        }
    }

    fn suggestion(parameter: &str, element: Option<u8>, value: Value) -> Suggestion {
        Suggestion {
            parameter: parameter.to_owned(),
            part: None,
            element,
            value,
            reason: "because".to_owned(),
        }
    }

    #[test]
    fn parses_json_inside_extra_text() {
        let proposal = Proposal::parse(
            "Sure!\n```json\n{\"summary\":\"Bright\",\"changes\":[{\"parameter\":\"part.volume\",\"value\":100}]}\n```",
        )
        .unwrap();
        assert_eq!(proposal.summary, "Bright");
        assert_eq!(proposal.changes[0].element, None);
        assert_eq!(proposal.changes[0].reason, "");
        assert!(Proposal::parse("no json here").is_err());
        assert!(Proposal::parse("{ not json }").is_err());
        assert_eq!(Proposal::parse("{}").unwrap().changes, Vec::new());
    }

    #[test]
    fn resolves_changes_to_validated_locations() {
        let device = ModxPlus::new();
        let sound = sound(&device);
        let context = context(&device, &sound);
        let proposal = Proposal {
            summary: String::new(),
            lookup: Vec::new(),
            load: Vec::new(),
            changes: vec![
                suggestion("part.reverb-send", None, json!(40)),
                suggestion("effects.variation-type", None, json!("spx chorus")),
                suggestion("awm2.element-level", Some(2), json!(99.6)),
                suggestion("part.eq-high-gain", None, json!("8")),
                suggestion("awm2.wave-number", Some(1), json!("ep1 sw")),
                suggestion("awm2.wave-number", Some(2), json!(264)),
            ],
        };
        let resolved = resolve(&context, &proposal);
        assert!(
            resolved.iter().all(Resolved::is_applicable),
            "{resolved:#?}"
        );
        assert_eq!(
            resolved[0].change,
            Some(ParameterChange::new(
                part::REVERB_SEND,
                Location::part(3),
                40
            ))
        );
        assert_eq!(resolved[0].from.as_deref(), Some("12"));
        assert_eq!(resolved[0].to.as_deref(), Some("40"));
        assert_eq!(resolved[0].reason, "because");
        assert_eq!(
            resolved[1].change,
            Some(ParameterChange::new(
                effects::VARIATION_TYPE,
                Location::performance(),
                17
            ))
        );
        assert_eq!(resolved[1].to.as_deref(), Some("SPX CHORUS"));
        assert_eq!(resolved[2].name, "Element Level · Element 2");
        assert_eq!(
            resolved[2].change.map(|change| change.location),
            Some(Location::element(3, 2))
        );
        assert_eq!(resolved[2].change.map(|change| change.value), Some(100));
        assert_eq!(resolved[3].to.as_deref(), Some("+3.00 dB"));
        assert_eq!(resolved[4].to.as_deref(), Some("166 EP1 Sw"));
        assert_eq!(resolved[4].from.as_deref(), Some("1 CF3 Stretch Sw St"));
        assert_eq!(resolved[5].to.as_deref(), Some("264 DX1"));
    }

    #[test]
    fn rejects_what_cannot_be_applied() {
        let device = ModxPlus::new();
        let sound = sound(&device);
        let context = context(&device, &sound);
        let proposal = Proposal {
            summary: String::new(),
            lookup: Vec::new(),
            load: Vec::new(),
            changes: vec![
                suggestion("awm2.filter-type", Some(1), json!(1)),
                suggestion("made.up", None, json!(1)),
                suggestion("part.volume", None, json!(500)),
                suggestion("awm2.element-level", None, json!(10)),
                suggestion("part.pan", None, json!(true)),
                suggestion("part.reverb-send", None, json!(12)),
                suggestion("effects.reverb-type", None, json!("not a reverb")),
                suggestion("awm2.wave-number", Some(1), json!(9000)),
            ],
        };
        let resolved = resolve(&context, &proposal);
        let problems: Vec<_> = resolved
            .iter()
            .map(|row| row.problem.clone().unwrap_or_default())
            .collect();
        assert_eq!(problems[0], "SynthPilot can't edit this parameter yet.");
        assert!(problems[7].contains("9000"));
        assert_eq!(problems[1], "This parameter doesn't exist for this Part.");
        assert!(problems[2].contains("500"));
        assert_eq!(problems[3], "Choose which element to change.");
        assert_eq!(problems[4], "\"true\" is not a valid value.");
        assert_eq!(problems[5], "Already set to this value.");
        assert!(problems[6].contains("not a valid value"));
        assert!(resolved.iter().all(|row| !row.is_applicable()));
        assert!(resolved[5].change.is_some());
    }

    #[test]
    fn effect_parameters_follow_the_planned_type() {
        let device = ModxPlus::new();
        let sound = sound(&device);
        let context = context(&device, &sound);
        let proposal = Proposal {
            summary: String::new(),
            lookup: Vec::new(),
            load: Vec::new(),
            changes: vec![
                suggestion("effects.variation-parameter-2", None, json!(90)),
                suggestion("effects.variation-type", None, json!("SPX CHORUS")),
                suggestion("part.insertion-a-parameter-1", None, json!(20)),
                suggestion("effects.reverb-parameter-1", None, json!(500)),
            ],
        };
        let resolved = resolve(&context, &proposal);
        assert_eq!(resolved[0].name, "Variation · LFO Depth");
        assert!(resolved[0].is_applicable(), "{:?}", resolved[0]);
        assert!(resolved[1].is_applicable());
        assert_eq!(
            resolved[2].problem.as_deref(),
            Some("This parameter isn't used by the chosen effect type.")
        );
        assert!(
            resolved[3]
                .problem
                .as_deref()
                .unwrap_or_default()
                .contains("500")
        );
        assert_eq!(resolved[3].name, "Reverb · Reverb Time");
    }

    #[test]
    fn changes_go_to_the_part_the_model_names() {
        let device = ModxPlus::new();
        let mut sound = sound(&device);
        let engine = device.engine(awm2::ENGINE_ID).unwrap();
        sound
            .load(6, engine, Some("PAD/CHOIR".to_owned()), &[])
            .unwrap();
        let mut context = context(&device, &sound);
        context.parts.push(PartContext {
            number: 6,
            engine,
            part: sound.part(6).unwrap(),
        });
        let on =
            |parameter: &str, part: Option<u8>, element: Option<u8>, value: Value| Suggestion {
                part,
                ..suggestion(parameter, element, value)
            };
        let proposal = Proposal {
            summary: String::new(),
            lookup: Vec::new(),
            load: Vec::new(),
            changes: vec![
                on("part.volume", Some(6), None, json!(70)),
                on("awm2.element-level", Some(6), Some(1), json!(90)),
                on("part.volume", None, None, json!(110)),
                on("part.volume", Some(9), None, json!(70)),
                on("effects.reverb-return", Some(6), None, json!(80)),
            ],
        };
        let resolved = resolve(&context, &proposal);
        assert_eq!(
            resolved[0].change,
            Some(ParameterChange::new(part::VOLUME, Location::part(6), 70))
        );
        assert_eq!(resolved[0].name, "Part 6 · Volume");
        assert_eq!(
            resolved[1].change.map(|change| change.location),
            Some(Location::element(6, 1))
        );
        assert_eq!(resolved[1].name, "Part 6 · Element Level · Element 1");
        assert_eq!(
            resolved[2].change.map(|change| change.location),
            Some(Location::part(3)),
            "no part means the Part open in the editor"
        );
        assert_eq!(
            resolved[3].problem.as_deref(),
            Some("Part 9 isn't in this Performance.")
        );
        assert_eq!(
            resolved[4].change.map(|change| change.location),
            Some(Location::performance())
        );
        assert_eq!(resolved[4].name, "Reverb Return");

        context.selected = None;
        let proposal = Proposal {
            summary: String::new(),
            lookup: Vec::new(),
            load: Vec::new(),
            changes: vec![on("part.volume", None, None, json!(70))],
        };
        assert_eq!(
            resolve(&context, &proposal)[0].problem.as_deref(),
            Some("Choose which Part to change.")
        );
    }

    #[test]
    fn loads_name_a_preset_by_number_or_name() {
        let device = ModxPlus::new();
        let sound = sound(&device);
        let context = context(&device, &sound);
        let load = |part: Option<u8>, sound: Value| LoadSuggestion {
            part,
            sound,
            reason: "bells".to_owned(),
        };
        let proposal = Proposal {
            summary: String::new(),
            lookup: Vec::new(),
            changes: vec![Suggestion {
                part: Some(7),
                ..suggestion("part.volume", None, json!(90))
            }],
            load: vec![
                load(Some(7), json!(955)),
                load(None, json!("cfx concert")),
                load(Some(17), json!(3)),
                load(Some(2), json!(99_999)),
                load(Some(2), json!("No Such Sound")),
            ],
        };
        let resolved = resolve(&context, &proposal);
        assert_eq!(resolved[0].load, Some((7, 955)));
        assert_eq!(resolved[0].from.as_deref(), Some("Empty"));
        assert!(resolved[0].is_applicable());
        assert_eq!(resolved[1].load, Some((3, 3)), "the open Part by default");
        assert_eq!(resolved[1].from.as_deref(), Some("Unnamed sound"));
        assert_eq!(resolved[2].problem.as_deref(), Some("There is no Part 17."));
        assert!(resolved[3].problem.as_deref().unwrap().contains("99999"));
        assert!(resolved[4].problem.is_some());
        assert_eq!(
            resolved[5].problem.as_deref(),
            Some("Part 7 isn't in this Performance.")
        );
    }
}
