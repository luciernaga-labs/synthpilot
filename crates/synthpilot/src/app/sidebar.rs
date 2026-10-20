use eframe::egui::{self, RichText, text::LayoutJob};
use synthpilot_core::domain::{EngineProfile, Sound};

use super::state::{Action, Workspace};
use super::theme;

const ROW_HEIGHT: f32 = 28.0;

pub(crate) fn show(
    ui: &mut egui::Ui,
    workspace: &Workspace,
    reading: Option<(usize, usize)>,
    status: Option<&str>,
    actions: &mut Vec<Action>,
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        theme::section(ui, "Performance");
        let name = workspace.sound().and_then(Sound::name);
        match name {
            Some(name) => ui.label(RichText::new(name).heading().strong()),
            None if reading.is_some() => ui.label(RichText::new("Reading…").heading().weak()),
            None => ui.label(RichText::new("Untitled Performance").heading().weak()),
        };
        if let Some(status) = status
            && reading.is_none()
        {
            ui.weak(status);
        }
        ui.add_space(4.0);
        keyboard_controls(ui, reading, actions);

        if let Some(sound) = workspace.sound() {
            parts(ui, workspace, sound, actions);
        }
    });
}

fn parts(ui: &mut egui::Ui, workspace: &Workspace, sound: &Sound, actions: &mut Vec<Action>) {
    theme::section(ui, "Parts");
    let (used, empty): (Vec<u8>, Vec<u8>) =
        (1..=sound.part_count()).partition(|number| sound.part(*number).is_some());
    if used.is_empty() {
        ui.weak("This Performance has no Parts SynthPilot can edit.");
    }
    for number in used {
        part_row(ui, workspace, sound, number, actions);
    }
    if !empty.is_empty() {
        ui.add_space(4.0);
        let title = match empty.len() {
            1 => "1 empty Part".to_owned(),
            count => format!("{count} empty Parts"),
        };
        egui::CollapsingHeader::new(RichText::new(title).weak())
            .id_salt("empty_parts")
            .default_open(empty.contains(&workspace.part()))
            .show(ui, |ui| {
                for number in empty {
                    let selected = workspace.part() == number;
                    let text = RichText::new(format!("{number:>2}  empty")).weak();
                    if ui
                        .with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                            ui.add(
                                egui::Button::selectable(selected, text)
                                    .min_size(egui::vec2(0.0, ROW_HEIGHT - 6.0)),
                            )
                        })
                        .inner
                        .clicked()
                    {
                        actions.push(Action::SelectPart(number));
                    }
                }
            });
    }
}

fn part_row(
    ui: &mut egui::Ui,
    workspace: &Workspace,
    sound: &Sound,
    number: u8,
    actions: &mut Vec<Action>,
) {
    let part = sound.part(number);
    let engine = part
        .and_then(|part| workspace.device().engine(part.engine()))
        .map_or("—", EngineProfile::name);
    let name = part.and_then(|part| part.name()).unwrap_or("Unnamed");
    let selected = workspace.part() == number;

    let style = ui.style();
    let body = egui::TextStyle::Body.resolve(style);
    let small = egui::TextStyle::Small.resolve(style);
    let text_color = if selected {
        egui::Color32::WHITE
    } else {
        style.visuals.text_color()
    };
    let mut job = LayoutJob::default();
    job.append(
        &format!("{number:>2}  "),
        0.0,
        egui::TextFormat::simple(
            egui::FontId::monospace(body.size),
            style.visuals.weak_text_color(),
        ),
    );
    job.append(
        engine,
        0.0,
        egui::TextFormat::simple(small, theme::engine_color(engine)),
    );
    job.append(name, 8.0, egui::TextFormat::simple(body, text_color));

    let response = ui
        .with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
            ui.add(egui::Button::selectable(selected, job).min_size(egui::vec2(0.0, ROW_HEIGHT)))
        })
        .inner
        .on_hover_text(format!("Part {number} · {engine} · {name}"));
    if response.clicked() {
        actions.push(Action::SelectPart(number));
    }
}

fn keyboard_controls(
    ui: &mut egui::Ui,
    reading: Option<(usize, usize)>,
    actions: &mut Vec<Action>,
) {
    match reading {
        Some((finished, requested)) => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(format!("Reading the Performance… {finished}/{requested}"));
            });
        }
        None => {
            if ui
                .button("Read from keyboard")
                .on_hover_text(
                    "Read the Performance selected on the keyboard again. Use this after choosing \
                     another Performance on the MODX+.",
                )
                .clicked()
            {
                actions.push(Action::ReadFromKeyboard);
            }
        }
    }
}
