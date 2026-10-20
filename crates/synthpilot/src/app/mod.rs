mod controller;
mod editor;
mod midi;
mod preview;
mod sidebar;
mod state;
mod theme;

use std::time::{Duration, Instant};

use controller::Controller;
use eframe::egui::{self, RichText};
use synthpilot_core::devices::{self, DeviceDescriptor};
use synthpilot_core::domain::DeviceId;

use midi::{MidiAction, MidiLink};
use state::{Action, Workspace};

const LIVE_REPAINT: Duration = Duration::from_millis(100);
const IDLE_REPAINT: Duration = Duration::from_secs(1);
const CARD_WIDTH: f32 = 460.0;
const FIELD_WIDTH: f32 = 260.0;

pub(crate) struct SynthPilotApp {
    controller: Controller,
    chosen_device: Option<DeviceId>,
}

impl Default for SynthPilotApp {
    fn default() -> Self {
        Self {
            controller: Controller::new(MidiLink::new()),
            chosen_device: devices::catalog().first().map(|descriptor| descriptor.id),
        }
    }
}

pub(crate) fn apply_theme(ctx: &egui::Context) {
    theme::apply(ctx);
}

impl eframe::App for SynthPilotApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.controller.tick(Instant::now());
        let repaint = if self.controller.midi().live_port().is_some() {
            LIVE_REPAINT
        } else {
            IDLE_REPAINT
        };
        ui.ctx().request_repaint_after(repaint);

        let mut actions = Vec::new();
        let mut midi_actions = Vec::new();

        let controller = &self.controller;
        let workspace = controller.workspace();
        let midi = controller.midi();

        egui::Panel::top("top_bar").show(ui, |ui| {
            top_bar(ui, workspace, midi, &mut midi_actions);
        });

        match workspace {
            None => {
                let chosen = &mut self.chosen_device;
                egui::CentralPanel::default().show(ui, |ui| {
                    welcome(ui, controller, chosen, &mut actions, &mut midi_actions);
                });
            }
            Some(workspace) => {
                egui::Panel::left("sidebar")
                    .resizable(false)
                    .exact_size(250.0)
                    .show(ui, |ui| {
                        sidebar::show(
                            ui,
                            workspace,
                            controller.reading(),
                            controller.status(),
                            &mut actions,
                        );
                    });
                egui::Panel::bottom("midi_status")
                    .resizable(false)
                    .show(ui, |ui| preview::show(ui, workspace, controller));
                egui::CentralPanel::default().show(ui, |ui| {
                    editor::show(ui, workspace, controller, &mut actions);
                });
            }
        }

        self.controller
            .dispatch(actions, midi_actions, Instant::now());
    }
}

fn top_bar(
    ui: &mut egui::Ui,
    workspace: Option<&Workspace>,
    midi: &MidiLink,
    midi_actions: &mut Vec<MidiAction>,
) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.heading(RichText::new("SynthPilot").strong());
        if let Some(workspace) = workspace {
            ui.add_space(12.0);
            ui.label(RichText::new(workspace.device().display_name()).weak());
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if workspace.is_some() && midi.live_port().is_some() {
                if ui
                    .button("Disconnect")
                    .on_hover_text("Stop sending changes to the keyboard. Changes you haven't stored on the MODX+ are lost when you select another Performance there.")
                    .clicked()
                {
                    midi_actions.push(MidiAction::Disconnect);
                }
                ui.add_space(8.0);
            }
            theme::status_badge(ui, midi.live_port(), midi.sent());
        });
    });
    ui.add_space(6.0);
}

fn port_selector(
    ui: &mut egui::Ui,
    midi: &MidiLink,
    midi_actions: &mut Vec<MidiAction>,
    width: f32,
) -> egui::Response {
    let selected_text = midi
        .selected()
        .map_or_else(|| "No keyboard detected".to_owned(), ToString::to_string);
    egui::ComboBox::from_id_salt("midi_port")
        .width(width)
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            for port in midi.ports() {
                let selected = midi.selected() == Some(port);
                if ui.selectable_label(selected, &port.name).clicked() {
                    midi_actions.push(MidiAction::Select(port.clone()));
                }
            }
        })
        .response
        .on_hover_text("The list updates on its own when you plug in the keyboard. For the MODX+, choose the port named MODX Port 1.")
}

fn welcome(
    ui: &mut egui::Ui,
    controller: &Controller,
    chosen: &mut Option<DeviceId>,
    actions: &mut Vec<Action>,
    midi_actions: &mut Vec<MidiAction>,
) {
    let midi = controller.midi();
    ui.vertical_centered(|ui| {
        ui.add_space(ui.available_height() * 0.16);
        ui.set_max_width(CARD_WIDTH);
        egui::Frame::group(ui.style())
            .inner_margin(24.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.heading("Connect your keyboard");
                ui.weak("SynthPilot reads the Performance selected on your keyboard. Every change you make here is sent to the keyboard right away.");
                ui.add_space(16.0);

                egui::Grid::new("welcome_fields")
                    .num_columns(2)
                    .spacing([16.0, 12.0])
                    .show(ui, |ui| {
                        let label = ui.label("1. Synthesizer");
                        synth_picker(ui, chosen).labelled_by(label.id);
                        ui.end_row();

                        let label = ui.label("2. MIDI port");
                        port_selector(ui, midi, midi_actions, FIELD_WIDTH - 8.0)
                            .labelled_by(label.id);
                        ui.end_row();
                    });

                ui.add_space(10.0);
                if midi.ports().is_empty() {
                    theme::banner(
                        ui,
                        theme::WARNING,
                        "No keyboard detected. Connect the keyboard's USB TO HOST port to your computer and turn the keyboard on. It appears here automatically.",
                        |_| {},
                    );
                    ui.add_space(6.0);
                }
                let errors: Vec<&str> = [midi.error(), controller.error()]
                    .into_iter()
                    .flatten()
                    .collect();
                if !errors.is_empty() {
                    ui.label(RichText::new("Couldn't connect").strong().color(theme::ERROR));
                    theme::errors(ui, &errors);
                }
                ui.add_space(8.0);

                let ready = chosen.is_some() && midi.selected().is_some();
                let connect = egui::Button::new(RichText::new("Connect and read Performance").size(16.0));
                let response = ui
                    .add_enabled(ready, connect.min_size(egui::vec2(FIELD_WIDTH, 36.0)))
                    .on_hover_text("Press Enter to connect.")
                    .on_disabled_hover_text("Choose a synthesizer and a MIDI port first.");
                let enter = ready
                    && !ui.ctx().egui_wants_keyboard_input()
                    && ui.input(|input| input.key_pressed(egui::Key::Enter));
                if (response.clicked() || enter)
                    && let Some(device) = *chosen
                {
                    midi_actions.push(MidiAction::Connect);
                    actions.push(Action::SelectDevice(device));
                }
                ui.add_space(10.0);
                ui.weak("Before you connect: on the MODX+, go to Utility > Settings > MIDI I/O and set MIDI IN/OUT to USB.");
            });
    });
}

fn synth_picker(ui: &mut egui::Ui, chosen: &mut Option<DeviceId>) -> egui::Response {
    let selected_text = chosen
        .and_then(|id| {
            devices::catalog()
                .iter()
                .find(|descriptor| descriptor.id == id)
        })
        .map_or_else(
            || "Select a synthesizer".to_owned(),
            DeviceDescriptor::display_name,
        );
    egui::ComboBox::from_id_salt("welcome_synth")
        .width(FIELD_WIDTH - 8.0)
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            for descriptor in devices::catalog() {
                ui.selectable_value(chosen, Some(descriptor.id), descriptor.display_name());
            }
        })
        .response
}
