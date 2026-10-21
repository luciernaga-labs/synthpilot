use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Align2, Color32, FontId, Key, RichText, Stroke};
use synthpilot_core::domain::{
    EngineProfile, ListedValue, ParameterDefinition, ParameterId, Part, Scope, Sound, Support,
    ValueFormat,
};

use super::controller::Controller;
use super::state::{Action, EFFECTS, Workspace};
use super::theme;

const CONTROL_WIDTH: f32 = 200.0;
const TWO_COLUMNS: f32 = 820.0;
const SENT_FADE: Duration = Duration::from_millis(700);
const QUIET_LEVEL: i32 = 20;
const OFF_ON: [&str; 2] = ["Off", "On"];
const TAB_SIZE: egui::Vec2 = egui::vec2(54.0, 46.0);

type Originals = Arc<HashMap<(u8, Option<u8>, ParameterId), i32>>;

#[derive(Clone, Copy)]
struct ElementStatus {
    on: Option<bool>,
    level: Option<i32>,
    max_level: i32,
}

impl ElementStatus {
    fn audible(self) -> bool {
        self.on != Some(false) && self.level.is_none_or(|level| level > 0)
    }
}

struct View<'a> {
    workspace: &'a Workspace,
    engine: &'a dyn EngineProfile,
    part: &'a Part,
    originals: &'a Originals,
}

impl View<'_> {
    fn element_for(&self, parameter: &ParameterDefinition) -> Option<u8> {
        match parameter.scope {
            Scope::Performance | Scope::Part => None,
            Scope::Element => Some(self.workspace.element()),
        }
    }
}

pub(crate) fn show(
    ui: &mut egui::Ui,
    workspace: &Workspace,
    controller: &Controller,
    actions: &mut Vec<Action>,
) {
    let midi = controller.midi();
    let errors: Vec<&str> = [workspace.error(), midi.error(), controller.error()]
        .into_iter()
        .flatten()
        .collect();
    theme::errors(ui, &errors);

    let reading = controller.reading();
    let just_read = reading_finished(ui, reading.is_some());

    let Some(sound) = workspace.sound() else {
        match reading {
            Some((finished, requested)) => reading_message(ui, finished, requested),
            None => no_sound(ui, actions),
        }
        return;
    };
    let originals = originals(ui, workspace, sound, reading.is_some(), just_read);

    let number = workspace.part();
    let current = if number == EFFECTS {
        sound.common()
    } else {
        sound.part(number)
    };
    let busy = reading.is_some() || controller.loading().is_some();
    let (Some(engine), Some(part)) = (workspace.engine(), current) else {
        empty_part(ui, workspace, sound, number, busy, actions);
        return;
    };

    header(ui, number, part, engine);
    if number != EFFECTS && !workspace.device().presets().is_empty() {
        egui::CollapsingHeader::new("Change sound…")
            .id_salt(("change_sound", number))
            .show(ui, |ui| {
                sound_picker(ui, workspace.device().presets(), number, busy, actions);
            });
        ui.add_space(6.0);
    }
    if let Some((finished, requested)) = reading {
        theme::banner(
            ui,
            theme::ACTIVITY,
            &format!("Reading the Performance from the keyboard… {finished} of {requested} Parts"),
            |ui| {
                ui.spinner();
            },
        );
        ui.add_space(6.0);
    }

    let view = View {
        workspace,
        engine,
        part,
        originals: &originals,
    };
    let has_elements = by_scope(engine, part, Scope::Element)
        .iter()
        .any(ParameterDefinition::is_implemented);
    if has_elements && reading.is_none() {
        auto_select_element(ui, &view, just_read, actions);
        element_shortcuts(ui, &view, actions);
    }

    ui.add_enabled_ui(reading.is_none(), |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                performance_section(ui, &view, actions);
                part_section(ui, &view, number, actions);
                ui.add_space(12.0);
                element_section(ui, &view, has_elements, actions);
                ui.add_space(12.0);
            });
    });
}

fn reading_finished(ui: &egui::Ui, reading: bool) -> bool {
    let id = egui::Id::unique("editor_was_reading");
    let was = ui.data(|data| data.get_temp::<bool>(id)).unwrap_or(false);
    ui.data_mut(|data| data.insert_temp(id, reading));
    was && !reading
}

fn originals(
    ui: &egui::Ui,
    workspace: &Workspace,
    sound: &Sound,
    reading: bool,
    just_read: bool,
) -> Originals {
    let id = egui::Id::unique("editor_originals");
    let stored = ui.data(|data| data.get_temp::<Originals>(id));
    if let Some(stored) = stored.filter(|_| !just_read || reading) {
        return stored;
    }
    let mut values = HashMap::new();
    if let (Some(common), Some(engine)) = (sound.common(), workspace.device().common()) {
        for parameter in engine.parameters().iter().filter(|p| p.is_implemented()) {
            if let Some(value) = common.value(parameter.id, None) {
                values.insert((EFFECTS, None, parameter.id), value);
            }
        }
    }
    for number in 1..=sound.part_count() {
        let Some(part) = sound.part(number) else {
            continue;
        };
        let Some(engine) = workspace.device().engine(part.engine()) else {
            continue;
        };
        for parameter in engine.parameters().iter().filter(|p| p.is_implemented()) {
            let elements: Vec<Option<u8>> = match parameter.scope {
                Scope::Performance | Scope::Part => vec![None],
                Scope::Element => (1..=engine.layout().elements).map(Some).collect(),
            };
            for element in elements {
                if let Some(value) = part.value(parameter.id, element) {
                    values.insert((number, element, parameter.id), value);
                }
            }
        }
    }
    let originals: Originals = Arc::new(values);
    ui.data_mut(|data| data.insert_temp(id, originals.clone()));
    originals
}

fn centered(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.vertical_centered(|ui| {
        ui.add_space(ui.available_height() * 0.25);
        add_contents(ui);
    });
}

fn reading_message(ui: &mut egui::Ui, finished: usize, requested: usize) {
    centered(ui, |ui| {
        ui.spinner();
        ui.add_space(8.0);
        ui.heading("Reading the Performance from the keyboard…");
        ui.weak("Keep the Performance you want to edit selected on the keyboard.");
        ui.add_space(10.0);
        let progress = if requested == 0 {
            0.0
        } else {
            ratio(finished, requested)
        };
        ui.add(
            egui::ProgressBar::new(progress)
                .desired_width(320.0)
                .text(format!("{finished} of {requested} Parts")),
        );
    });
}

fn no_sound(ui: &mut egui::Ui, actions: &mut Vec<Action>) {
    centered(ui, |ui| {
        ui.heading("No Performance yet");
        ui.weak("SynthPilot reads the Performance when you connect. If nothing appears, check the message above and select Read from keyboard.");
        ui.add_space(10.0);
        if ui
            .add(egui::Button::new("Read from keyboard").min_size(egui::vec2(200.0, 32.0)))
            .clicked()
        {
            actions.push(Action::ReadFromKeyboard);
        }
    });
}

fn empty_part(
    ui: &mut egui::Ui,
    workspace: &Workspace,
    sound: &Sound,
    number: u8,
    busy: bool,
    actions: &mut Vec<Action>,
) {
    let presets = workspace.device().presets();
    if number != EFFECTS && !presets.is_empty() {
        ui.add_space(4.0);
        ui.heading(RichText::new(format!("Part {number} is empty")).strong());
        ui.weak("Pick a sound from the keyboard's presets to add it to this Part.");
        ui.add_space(8.0);
        sound_picker(ui, presets, number, busy, actions);
        return;
    }
    centered(ui, |ui| {
        ui.heading(format!("Part {number} is empty"));
        ui.weak(format!("This Performance doesn't use Part {number}."));
        ui.add_space(10.0);
        let first = (1..=sound.part_count()).find(|n| sound.part(*n).is_some());
        if let Some(first) = first
            && ui
                .add(
                    egui::Button::new(format!("Go to Part {first}"))
                        .min_size(egui::vec2(200.0, 32.0)),
                )
                .clicked()
        {
            actions.push(Action::SelectPart(first));
        }
    });
}

/// A searchable list of the keyboard's preset sounds and a button that loads
/// the chosen one into `part`.
fn sound_picker(
    ui: &mut egui::Ui,
    presets: &'static [ListedValue],
    part: u8,
    busy: bool,
    actions: &mut Vec<Action>,
) {
    let search_id = egui::Id::unique(("sound_search", part));
    let chosen_id = egui::Id::unique(("sound_chosen", part));
    let mut search: String = ui.data(|data| data.get_temp(search_id)).unwrap_or_default();
    let mut chosen: Option<i32> = ui.data(|data| data.get_temp(chosen_id)).flatten();
    ui.add(
        egui::TextEdit::singleline(&mut search)
            .hint_text("Search a name or category, e.g. Bell, Pad/Choir, CFX")
            .desired_width(420.0),
    );
    let needle = search.trim().to_lowercase();
    let matches: Vec<(i32, &ListedValue)> = (1..)
        .zip(presets.iter())
        .filter(|(number, preset)| {
            needle.is_empty()
                || preset.name.to_lowercase().contains(&needle)
                || preset.group.to_lowercase().contains(&needle)
                || number.to_string() == needle
        })
        .collect();
    ui.weak(format!("{} of {} sounds", matches.len(), presets.len()));
    let row_height = ui.text_style_height(&egui::TextStyle::Body) + 6.0;
    egui::ScrollArea::vertical()
        .id_salt(("sound_list", part))
        .max_height(260.0)
        .show_rows(ui, row_height, matches.len(), |ui, rows| {
            for (number, preset) in &matches[rows] {
                let text = format!("{number:>4}  {}   {}", preset.name, preset.group);
                if ui.selectable_label(chosen == Some(*number), text).clicked() {
                    chosen = Some(*number);
                }
            }
        });
    ui.add_space(6.0);
    let label = match chosen.and_then(|number| presets.get(usize::try_from(number - 1).ok()?)) {
        Some(preset) => format!("Load {} into Part {part}", preset.name),
        None => format!("Choose a sound for Part {part}"),
    };
    let button = ui
        .add_enabled(
            chosen.is_some() && !busy,
            egui::Button::new(label).min_size(egui::vec2(240.0, 30.0)),
        )
        .on_hover_text(
            "Replaces this Part's sound and its insertion effects with the preset. For a preset with several Parts, the keyboard loads only the first one. Needs MIDI I/O Mode set to Multi on the keyboard.",
        );
    if button.clicked()
        && let Some(number) = chosen
    {
        actions.push(Action::LoadPreset { part, number });
    }
    if part == 1 {
        ui.weak("Part 1 holds the main sound of the Performance.");
    }
    ui.data_mut(|data| {
        data.insert_temp(search_id, search);
        data.insert_temp(chosen_id, chosen);
    });
}

fn header(ui: &mut egui::Ui, number: u8, part: &Part, engine: &dyn EngineProfile) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if number == EFFECTS {
            ui.heading(RichText::new("Performance effects").strong());
            return;
        }
        ui.heading(RichText::new(format!("Part {number}")).strong());
        if let Some(name) = part.name() {
            ui.heading(RichText::new(format!("· {name}")).strong());
        }
        ui.add_space(6.0);
        theme::engine_chip(ui, engine.name());
    });
    ui.weak(engine.description());
    ui.add_space(8.0);
}

fn by_scope(engine: &dyn EngineProfile, part: &Part, scope: Scope) -> Vec<ParameterDefinition> {
    engine
        .parameters()
        .iter()
        .filter(|parameter| parameter.scope == scope)
        .filter_map(|parameter| engine.refine(parameter, part))
        .collect()
}

fn switch_parameter(engine: &dyn EngineProfile) -> Option<&ParameterDefinition> {
    engine.parameters().iter().find(|parameter| {
        parameter.scope == Scope::Element
            && parameter.is_implemented()
            && matches!(parameter.format, ValueFormat::Choice(labels) if *labels == OFF_ON)
    })
}

fn level_parameter(engine: &dyn EngineProfile) -> Option<&ParameterDefinition> {
    engine.parameters().iter().find(|parameter| {
        parameter.scope == Scope::Element
            && parameter.is_implemented()
            && parameter.name.ends_with(" Level")
    })
}

fn element_status(view: &View<'_>, element: u8) -> ElementStatus {
    let switch = switch_parameter(view.engine);
    let level = level_parameter(view.engine);
    ElementStatus {
        on: switch
            .and_then(|parameter| view.part.value(parameter.id, Some(element)))
            .map(|value| value != 0),
        level: level.and_then(|parameter| view.part.value(parameter.id, Some(element))),
        max_level: level.map_or(127, |parameter| parameter.range.max()),
    }
}

fn auto_select_element(ui: &egui::Ui, view: &View<'_>, just_read: bool, actions: &mut Vec<Action>) {
    let id = egui::Id::unique("editor_auto_element");
    let part = view.workspace.part();
    let seen = ui.data(|data| data.get_temp::<u8>(id));
    if seen == Some(part) && !just_read {
        return;
    }
    ui.data_mut(|data| data.insert_temp(id, part));
    let current = view.workspace.element();
    if element_status(view, current).audible() {
        return;
    }
    let best = (1..=view.engine.layout().elements)
        .map(|element| (element, element_status(view, element)))
        .filter(|(_, status)| status.audible())
        .max_by_key(|(element, status)| (status.level.unwrap_or(0), std::cmp::Reverse(*element)))
        .map(|(element, _)| element);
    if let Some(best) = best
        && best != current
    {
        actions.push(Action::SelectElement(best));
    }
}

fn element_shortcuts(ui: &egui::Ui, view: &View<'_>, actions: &mut Vec<Action>) {
    if ui.ctx().egui_wants_keyboard_input() {
        return;
    }
    let count = view.engine.layout().elements;
    let current = view.workspace.element();
    let digits = [
        Key::Num1,
        Key::Num2,
        Key::Num3,
        Key::Num4,
        Key::Num5,
        Key::Num6,
        Key::Num7,
        Key::Num8,
        Key::Num9,
    ];
    ui.input(|input| {
        if input.key_pressed(Key::OpenBracket) && current > 1 {
            actions.push(Action::SelectElement(current - 1));
        }
        if input.key_pressed(Key::CloseBracket) && current < count {
            actions.push(Action::SelectElement(current + 1));
        }
        if input.modifiers.command {
            for (element, key) in (1..=count).zip(digits) {
                if input.key_pressed(key) {
                    actions.push(Action::SelectElement(element));
                }
            }
        }
    });
}

fn performance_section(ui: &mut egui::Ui, view: &View<'_>, actions: &mut Vec<Action>) {
    let owned = by_scope(view.engine, view.part, Scope::Performance);
    let parameters: Vec<&ParameterDefinition> = owned.iter().collect();
    if parameters.is_empty() {
        return;
    }
    theme::scope_frame(
        ui,
        theme::PART_ACCENT,
        "Performance",
        "Common: shared by every Part of the Performance.",
        |ui| parameter_groups(ui, view, &parameters, "performance", actions),
    );
}

fn part_section(ui: &mut egui::Ui, view: &View<'_>, number: u8, actions: &mut Vec<Action>) {
    let owned = by_scope(view.engine, view.part, Scope::Part);
    let parameters: Vec<&ParameterDefinition> = owned.iter().collect();
    if parameters.is_empty() {
        return;
    }
    theme::scope_frame(
        ui,
        theme::PART_ACCENT,
        &format!("Part {number}"),
        "Common: changes the whole sound of this Part.",
        |ui| parameter_groups(ui, view, &parameters, "part", actions),
    );
}

fn element_section(
    ui: &mut egui::Ui,
    view: &View<'_>,
    has_elements: bool,
    actions: &mut Vec<Action>,
) {
    let owned = by_scope(view.engine, view.part, Scope::Element);
    let parameters: Vec<&ParameterDefinition> = owned.iter().collect();
    if parameters.is_empty() {
        return;
    }
    let label = view.engine.element_label();
    if !has_elements {
        ui.weak(format!(
            "Parameters for each {} are not editable yet.",
            label.to_lowercase()
        ));
        unavailable(ui, &parameters, "element");
        return;
    }
    let element = view.workspace.element();
    let count = view.engine.layout().elements;
    theme::scope_frame(
        ui,
        theme::ELEMENT_ACCENT,
        &format!("{label} {element} of {count}"),
        "Only changes this layer of the sound.",
        |ui| {
            element_tabs(ui, view, actions);
            element_warning(ui, view, actions);
            parameter_groups(ui, view, &parameters, "element", actions);
        },
    );
}

fn element_tabs(ui: &mut egui::Ui, view: &View<'_>, actions: &mut Vec<Action>) {
    let label = view.engine.element_label();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        for element in 1..=view.engine.layout().elements {
            let status = element_status(view, element);
            let selected = view.workspace.element() == element;
            if element_tab(ui, label, element, selected, status).clicked() {
                actions.push(Action::SelectElement(element));
            }
        }
    });
    ui.weak("[ and ] switch layers. A dimmed layer is off or silent.");
    ui.add_space(6.0);
}

fn element_tab(
    ui: &mut egui::Ui,
    label: &str,
    element: u8,
    selected: bool,
    status: ElementStatus,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(TAB_SIZE, egui::Sense::click());
    let description = match (status.on, status.level) {
        (Some(false), _) => format!("{label} {element}: off — you won't hear it"),
        (_, Some(0)) => format!("{label} {element}: level 0 — you won't hear it"),
        (_, Some(level)) => format!("{label} {element}: on, level {level}"),
        _ => format!("{label} {element}"),
    };
    response
        .widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, selected, &description));
    let response = response.on_hover_text(&description);

    let audible = status.audible();
    let accent = theme::ELEMENT_ACCENT;
    let visuals = ui.visuals();
    let fill = if selected {
        accent.gamma_multiply(0.35)
    } else if response.hovered() {
        visuals.widgets.hovered.weak_bg_fill
    } else {
        visuals.widgets.inactive.weak_bg_fill
    };
    let stroke = if response.has_focus() {
        visuals.selection.stroke
    } else if selected {
        Stroke::new(1.5, accent)
    } else {
        Stroke::new(1.0, visuals.widgets.inactive.bg_stroke.color)
    };
    let text_color = match (selected, audible) {
        (true, _) => Color32::WHITE,
        (false, true) => visuals.text_color(),
        (false, false) => visuals.text_color().gamma_multiply(0.4),
    };
    let painter = ui.painter();
    painter.rect(rect, 6.0, fill, stroke, egui::StrokeKind::Inside);
    painter.text(
        rect.center_top() + egui::vec2(-4.0, 16.0),
        Align2::CENTER_CENTER,
        element.to_string(),
        FontId::proportional(15.0),
        text_color,
    );
    let dot = rect.right_top() + egui::vec2(-11.0, 12.0);
    match status.on {
        Some(true) => {
            painter.circle_filled(dot, 4.0, theme::SUCCESS);
        }
        Some(false) => {
            painter.circle_stroke(dot, 3.5, Stroke::new(1.2, theme::IDLE));
        }
        None => {}
    }
    let bar = egui::Rect::from_min_size(
        rect.left_bottom() + egui::vec2(8.0, -12.0),
        egui::vec2(rect.width() - 16.0, 5.0),
    );
    painter.rect_filled(bar, 2.0, Color32::from_white_alpha(18));
    if let Some(level) = status.level {
        let filled = bar.width() * ratio_i32(level, status.max_level);
        let color = if audible {
            accent
        } else {
            theme::IDLE.gamma_multiply(0.5)
        };
        painter.rect_filled(
            egui::Rect::from_min_size(bar.min, egui::vec2(filled, bar.height())),
            2.0,
            color,
        );
    }
    response
}

fn element_warning(ui: &mut egui::Ui, view: &View<'_>, actions: &mut Vec<Action>) {
    let label = view.engine.element_label();
    let element = view.workspace.element();
    let status = element_status(view, element);
    if status.on == Some(false) {
        theme::banner(
            ui,
            theme::WARNING,
            &format!("{label} {element} is off, so you won't hear changes made here."),
            |ui| {
                if let Some(switch) = switch_parameter(view.engine)
                    && ui.button("Turn it on").clicked()
                {
                    actions.push(Action::SetValue(switch.id, 1));
                }
            },
        );
        ui.add_space(6.0);
    } else if let Some(level) = status.level
        && level < QUIET_LEVEL
    {
        theme::banner(
            ui,
            theme::WARNING,
            &format!(
                "{label} {element} is very quiet (level {level}). Raise its level to hear your changes."
            ),
            |_| {},
        );
        ui.add_space(6.0);
    }
}

fn parameter_groups(
    ui: &mut egui::Ui,
    view: &View<'_>,
    parameters: &[&ParameterDefinition],
    scope_id: &str,
    actions: &mut Vec<Action>,
) {
    let available: Vec<&ParameterDefinition> = parameters
        .iter()
        .copied()
        .filter(|parameter| parameter.is_implemented())
        .collect();
    let mut categories: Vec<&str> = Vec::new();
    for parameter in &available {
        if !categories.contains(&parameter.category) {
            categories.push(parameter.category);
        }
    }
    let columns = if ui.available_width() >= TWO_COLUMNS {
        2
    } else {
        1
    };
    egui::Grid::new(("parameters", scope_id))
        .num_columns(columns * 2)
        .spacing([14.0, 8.0])
        .min_col_width(110.0)
        .show(ui, |ui| {
            for category in categories {
                ui.add_space(2.0);
                theme::heading(ui, category);
                ui.end_row();
                let in_category: Vec<&ParameterDefinition> = available
                    .iter()
                    .copied()
                    .filter(|parameter| parameter.category == category)
                    .collect();
                for chunk in in_category.chunks(columns) {
                    for parameter in chunk {
                        row(ui, view, parameter, actions);
                    }
                    ui.end_row();
                }
            }
        });
    let missing: Vec<&ParameterDefinition> = parameters
        .iter()
        .copied()
        .filter(|parameter| !parameter.is_implemented())
        .collect();
    if !missing.is_empty() {
        ui.add_space(6.0);
        unavailable(ui, &missing, scope_id);
    }
}

fn unavailable(ui: &mut egui::Ui, parameters: &[&ParameterDefinition], scope_id: &str) {
    let missing: Vec<&&ParameterDefinition> = parameters
        .iter()
        .filter(|parameter| !parameter.is_implemented())
        .collect();
    if missing.is_empty() {
        return;
    }
    let title = match missing.len() {
        1 => "1 parameter not editable yet".to_owned(),
        count => format!("{count} parameters not editable yet"),
    };
    egui::CollapsingHeader::new(RichText::new(title).weak())
        .id_salt(("unavailable", scope_id))
        .default_open(false)
        .show(ui, |ui| {
            for parameter in missing {
                if let Support::NotImplemented { reason } = parameter.support {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(parameter.name);
                        ui.weak(reason);
                    });
                }
            }
        });
}

fn display_name(parameter: &ParameterDefinition) -> String {
    if parameter.scope == Scope::Part && parameter.category.contains("Offset") {
        format!("± {}", parameter.name)
    } else {
        parameter.name.to_owned()
    }
}

fn hint(parameter: &ParameterDefinition) -> String {
    let mut text = parameter
        .help
        .map(|help| format!("{help}\n\n"))
        .unwrap_or_default();
    let _ = write!(text, "Range {}", parameter.display_range());
    if parameter.help.is_none()
        && parameter.scope == Scope::Part
        && parameter.category.contains("Offset")
    {
        text.push_str("\nShifts this setting for every Element of the Part at once. 0 keeps the sound as programmed.");
    }
    if let Some(default) = parameter.default {
        let _ = write!(text, "\nDefault {}", parameter.display_value(default));
    }
    text.push_str("\nRight-click the control to reset it.");
    if let Support::Implemented(source) = parameter.support {
        let _ = write!(text, "\n\n{source}");
    }
    text
}

fn row(
    ui: &mut egui::Ui,
    view: &View<'_>,
    parameter: &ParameterDefinition,
    actions: &mut Vec<Action>,
) {
    let hint = hint(parameter);
    let name = if view.workspace.selected() == Some(parameter.id) {
        RichText::new(display_name(parameter)).strong()
    } else {
        RichText::new(display_name(parameter))
    };
    let label = ui.label(name).on_hover_text(&hint);
    let element = view.element_for(parameter);
    let current = view.workspace.value(parameter);

    ui.horizontal(|ui| {
        let Some(current) = current else {
            ui.add_enabled(false, egui::Label::new("—"))
                .on_disabled_hover_text("This value has not been read from the keyboard.");
            return;
        };
        let mut value = current;
        let response = control(ui, parameter, &mut value)
            .labelled_by(label.id)
            .on_hover_text(&hint);
        let sent_id = egui::Id::unique(("sent", parameter.id, view.workspace.part(), element));
        if response.clicked() || response.drag_started() || response.gained_focus() {
            actions.push(Action::SelectParameter(parameter.id));
        }
        if value != current {
            actions.push(Action::SetValue(parameter.id, value));
            ui.data_mut(|data| data.insert_temp(sent_id, Instant::now()));
        }
        sent_marker(ui, sent_id);
        response.context_menu(|ui| {
            reset_menu(ui, view, parameter, element, current, actions);
        });
    });
}

fn reset_menu(
    ui: &mut egui::Ui,
    view: &View<'_>,
    parameter: &ParameterDefinition,
    element: Option<u8>,
    current: i32,
    actions: &mut Vec<Action>,
) {
    let original = view
        .originals
        .get(&(view.workspace.part(), element, parameter.id))
        .copied();
    let mut offered = false;
    if let Some(original) = original {
        offered = true;
        let text = format!(
            "Restore keyboard value ({})",
            parameter.display_value(original)
        );
        if ui
            .add_enabled(original != current, egui::Button::new(text))
            .clicked()
        {
            actions.push(Action::SetValue(parameter.id, original));
            ui.close();
        }
    }
    if let Some(default) = parameter.default {
        offered = true;
        let text = format!("Reset to default ({})", parameter.display_value(default));
        if ui
            .add_enabled(default != current, egui::Button::new(text))
            .clicked()
        {
            actions.push(Action::SetValue(parameter.id, default));
            ui.close();
        }
    }
    if !offered {
        ui.weak("No reset value for this setting.");
    }
}

fn sent_marker(ui: &mut egui::Ui, id: egui::Id) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
    let Some(at) = ui.data(|data| data.get_temp::<Instant>(id)) else {
        return;
    };
    let elapsed = at.elapsed();
    if elapsed >= SENT_FADE {
        return;
    }
    let fade = 1.0 - elapsed.as_secs_f32() / SENT_FADE.as_secs_f32();
    ui.painter()
        .circle_filled(rect.center(), 4.0, theme::ACTIVITY.gamma_multiply(fade));
    ui.ctx().request_repaint();
}

fn control(ui: &mut egui::Ui, parameter: &ParameterDefinition, value: &mut i32) -> egui::Response {
    match parameter.format {
        ValueFormat::Choice(labels) if *labels == OFF_ON => {
            let off = ui.selectable_label(*value == 0, "Off");
            let on = ui.selectable_label(*value != 0, "On");
            if off.clicked() {
                *value = 0;
            }
            if on.clicked() {
                *value = 1;
            }
            off | on
        }
        ValueFormat::Listed { first, values } => listed(ui, parameter, value, first, values),
        ValueFormat::Choice(_) => {
            egui::ComboBox::from_id_salt(parameter.id.as_str())
                .width(CONTROL_WIDTH)
                .height(320.0)
                .selected_text(match parameter.group_of(*value) {
                    Some(group) => format!("{}  ·  {group}", parameter.display_value(*value)),
                    None => parameter.display_value(*value),
                })
                .show_ui(ui, |ui| {
                    for option in parameter.range.min()..=parameter.range.max() {
                        if let Some(group) = parameter.group_starting_at(option) {
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new(group).small().strong().weak());
                        }
                        ui.selectable_value(value, option, parameter.display_value(option));
                    }
                })
                .response
        }
        ValueFormat::Number
        | ValueFormat::Signed
        | ValueFormat::Pan
        | ValueFormat::Scaled { .. }
        | ValueFormat::Note => slider(ui, parameter, value),
    }
}

fn listed(
    ui: &mut egui::Ui,
    parameter: &ParameterDefinition,
    value: &mut i32,
    first: i32,
    values: &[ListedValue],
) -> egui::Response {
    let search_id = egui::Id::unique(("listed_search", parameter.id.as_str()));
    egui::ComboBox::from_id_salt(parameter.id.as_str())
        .width(CONTROL_WIDTH)
        .height(360.0)
        .selected_text(parameter.display_value(*value))
        .show_ui(ui, |ui| {
            let mut search: String = ui.data(|data| data.get_temp(search_id)).unwrap_or_default();
            ui.add(
                egui::TextEdit::singleline(&mut search)
                    .hint_text("Search name or category, e.g. EP or Keys/Rd")
                    .desired_width(320.0),
            );
            ui.data_mut(|data| data.insert_temp(search_id, search.clone()));
            let needle = search.trim().to_lowercase();
            let matches: Vec<(i32, &ListedValue)> = (first..)
                .zip(values.iter())
                .filter(|(number, listed)| {
                    needle.is_empty()
                        || listed.name.to_lowercase().contains(&needle)
                        || listed.group.to_lowercase().contains(&needle)
                        || number.to_string() == needle
                })
                .collect();
            ui.weak(format!("{} of {}", matches.len(), values.len()));
            let row_height = ui.text_style_height(&egui::TextStyle::Body) + 4.0;
            egui::ScrollArea::vertical().max_height(300.0).show_rows(
                ui,
                row_height,
                matches.len(),
                |ui, rows| {
                    for (number, listed) in &matches[rows] {
                        let text = format!("{number} {}   {}", listed.name, listed.group);
                        ui.selectable_value(value, *number, text);
                    }
                },
            );
        })
        .response
}

fn slider(ui: &mut egui::Ui, parameter: &ParameterDefinition, value: &mut i32) -> egui::Response {
    let definition = *parameter;
    let min = parameter.range.min();
    let max = parameter.range.max();
    let before = *value;
    ui.spacing_mut().slider_width = CONTROL_WIDTH;
    let response = ui.add(
        egui::Slider::new(value, min..=max)
            .show_value(true)
            .custom_formatter(move |number, _| definition.display_value(round(number)))
            .custom_parser(move |text| parse(&definition, text).map(f64::from)),
    );
    let secondary = ui.input(|input| {
        input.pointer.button_down(egui::PointerButton::Secondary)
            || input
                .pointer
                .button_released(egui::PointerButton::Secondary)
    });
    if secondary {
        *value = before;
    }
    if min < 0 && max > 0 {
        center_mark(ui, &response, min, max);
    }
    let rail = egui::Rect::from_min_size(
        response.rect.min,
        egui::vec2(CONTROL_WIDTH, response.rect.height()),
    );
    let menu = ui.interact(rail, response.id.with("reset"), egui::Sense::click());
    response | menu
}

fn center_mark(ui: &egui::Ui, response: &egui::Response, min: i32, max: i32) {
    let thickness = ui
        .text_style_height(&egui::TextStyle::Body)
        .max(ui.spacing().interact_size.y);
    let radius = thickness / 2.5;
    let left = response.rect.left() + radius;
    let right = response.rect.left() + CONTROL_WIDTH - radius;
    let x = left + (right - left) * ratio_i32(-min, max - min);
    let middle = response.rect.center().y;
    let color = ui.visuals().weak_text_color();
    let stroke = Stroke::new(1.5, color);
    ui.painter().vline(
        x,
        egui::Rangef::new(middle - thickness / 2.0, middle - radius),
        stroke,
    );
    ui.painter().vline(
        x,
        egui::Rangef::new(middle + radius, middle + thickness / 2.0),
        stroke,
    );
}

fn parse(parameter: &ParameterDefinition, text: &str) -> Option<i32> {
    let mut text = text.trim().to_uppercase();
    if let Some(unit) = parameter.unit {
        let suffix = unit.suffix().to_uppercase();
        if let Some(stripped) = text.strip_suffix(&suffix) {
            text = stripped.trim().to_owned();
        }
    }
    let number = match parameter.format {
        ValueFormat::Pan => match text.as_str() {
            "C" => Some(0),
            _ => {
                if let Some(rest) = text.strip_prefix('L') {
                    rest.trim().parse::<i32>().ok().map(|number| -number)
                } else if let Some(rest) = text.strip_prefix('R') {
                    rest.trim().parse::<i32>().ok()
                } else {
                    text.parse::<i32>().ok()
                }
            }
        },
        ValueFormat::Scaled { thousandths, .. } if thousandths > 0 => text
            .parse::<f64>()
            .ok()
            .map(|number| round(number * 1000.0 / f64::from(thousandths))),
        ValueFormat::Note => parameter
            .value_named(&text)
            .or_else(|| text.parse::<i32>().ok()),
        _ => text.parse::<i32>().ok(),
    }?;
    parameter.range.contains(number).then_some(number)
}

#[expect(clippy::cast_possible_truncation)]
fn round(number: f64) -> i32 {
    number.round() as i32
}

#[expect(clippy::cast_possible_truncation)]
fn ratio_i32(value: i32, total: i32) -> f32 {
    if total <= 0 {
        return 0.0;
    }
    (f64::from(value.clamp(0, total)) / f64::from(total)) as f32
}

#[expect(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn ratio(value: usize, total: usize) -> f32 {
    (value.min(total) as f64 / total as f64) as f32
}
