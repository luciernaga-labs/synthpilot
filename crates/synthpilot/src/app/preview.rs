use eframe::egui::{self, RichText};
use synthpilot_core::domain::{EncodeError, Support};
use synthpilot_core::protocol::{SysExMessage, hex};

use super::controller::Controller;
use super::state::{Preview, Workspace};
use super::theme;

pub(crate) fn show(ui: &mut egui::Ui, workspace: &Workspace, controller: &Controller) {
    let midi = controller.midi();
    let open_id = egui::Id::unique("show_midi_details");
    let mut open = ui
        .data(|data| data.get_temp::<bool>(open_id))
        .unwrap_or(false);
    let preview = workspace.preview();

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let (text, color) = if midi.live_port().is_some() {
            ("Live", theme::SUCCESS)
        } else {
            ("Not connected — nothing is sent", theme::IDLE)
        };
        let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 4.0, color);
        ui.label(RichText::new(text).color(color));
        ui.weak(format!("· {} messages sent", midi.sent()));
        if let Some(preview) = &preview {
            ui.weak(format!("· editing: {}", last_change(workspace, preview)));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let label = if open {
                "Hide MIDI details"
            } else {
                "Show MIDI details"
            };
            if ui
                .selectable_label(open, label)
                .on_hover_text(
                    "The exact MIDI message (SysEx) SynthPilot sends for the selected parameter.",
                )
                .clicked()
            {
                open = !open;
            }
        });
    });
    ui.data_mut(|data| data.insert_temp(open_id, open));

    if open {
        ui.separator();
        match &preview {
            None => {
                ui.weak("Move a control to see the MIDI message SynthPilot sends for it.");
            }
            Some(preview) => details(ui, workspace, preview),
        }
    }
    ui.add_space(4.0);
}

fn last_change(workspace: &Workspace, preview: &Preview) -> String {
    let value = workspace.value(&preview.definition).map_or_else(
        || "—".to_owned(),
        |value| preview.definition.display_value(value),
    );
    format!("{} = {value}", location_label(workspace, preview))
}

fn location_label(workspace: &Workspace, preview: &Preview) -> String {
    let part = preview.location.part_number();
    match (preview.location.element_number(), workspace.engine()) {
        (Some(element), Some(engine)) => format!(
            "Part {part} · {} {element} · {}",
            engine.element_label(),
            preview.definition.name
        ),
        _ => format!("Part {part} · {}", preview.definition.name),
    }
}

fn details(ui: &mut egui::Ui, workspace: &Workspace, preview: &Preview) {
    ui.label(RichText::new(location_label(workspace, preview)).strong());
    match &preview.result {
        Ok(message) => {
            message_bytes(ui, message);
            if let Support::Implemented(source) = preview.definition.support {
                ui.weak(format!("Source: {source}"));
            }
        }
        Err(EncodeError::NotImplemented(_)) => {
            if let Support::NotImplemented { reason } = preview.definition.support {
                ui.label(RichText::new("Not editable yet").color(theme::WARNING));
                ui.weak(reason);
            }
        }
        Err(error) => {
            ui.label(RichText::new(error.to_string()).color(theme::ERROR));
        }
    }
}

fn message_bytes(ui: &mut egui::Ui, message: &SysExMessage) {
    ui.horizontal_wrapped(|ui| {
        for segment in message.segments() {
            ui.vertical(|ui| {
                let bytes = &message.as_bytes()[segment.range.clone()];
                ui.label(RichText::new(hex(bytes)).monospace());
                ui.label(RichText::new(segment.label).small().weak());
            });
            ui.add_space(8.0);
        }
        if ui
            .button("Copy")
            .on_hover_text("Copy this message as text, for example to include in a bug report.")
            .clicked()
        {
            ui.ctx().copy_text(message.to_hex());
        }
    });
}
