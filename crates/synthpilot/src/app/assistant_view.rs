use eframe::egui::{self, RichText};

use super::assistant::{AssistantPanel, IDEAS, Setup};
use super::state::{Action, Workspace};
use super::theme;

const ACCENT: egui::Color32 = theme::ELEMENT_ACCENT;

pub(crate) fn show(
    ui: &mut egui::Ui,
    panel: &mut AssistantPanel,
    workspace: &Workspace,
    reading: bool,
    actions: &mut Vec<Action>,
) {
    theme::section(ui, "Assistant");
    match panel.setup() {
        Setup::Ready(_) => {}
        Setup::Missing => {
            setup_help(ui, None);
            return;
        }
        Setup::Invalid(problem) => {
            let problem = problem.clone();
            setup_help(ui, Some(&problem));
            return;
        }
    }
    if let Some(model) = panel.model() {
        ui.label(RichText::new(model).small().weak());
    }
    ui.add_space(6.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            prompt(ui, panel, workspace, reading);
            ui.add_space(8.0);
            if let Some(request) = panel.waiting_for() {
                let text = format!("Designing \"{request}\"…");
                theme::banner(ui, theme::ACTIVITY, &text, |ui| {
                    ui.spinner();
                });
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(100));
            }
            if let Some(error) = panel.error() {
                theme::banner(ui, theme::ERROR, error, |_| {});
            }
            if let Some(message) = panel.message() {
                theme::banner(ui, theme::SUCCESS, message, |_| {});
            }
            plan(ui, panel, workspace, actions);
            if panel.can_undo() && panel.plan().is_none() {
                ui.add_space(6.0);
                if ui
                    .button("Undo")
                    .on_hover_text("Put back the values from before the last Apply.")
                    .clicked()
                {
                    actions.extend(panel.undo());
                }
            }
        });
}

fn setup_help(ui: &mut egui::Ui, problem: Option<&str>) {
    if let Some(problem) = problem {
        theme::banner(ui, theme::WARNING, problem, |_| {});
        ui.add_space(6.0);
    }
    ui.label("Describe a sound and let your favorite AI model set the parameters.");
    ui.add_space(4.0);
    ui.weak("Copy .env.example to .env in the folder you start SynthPilot from, fill it in and restart:");
    ui.add_space(4.0);
    ui.monospace("SYNTHPILOT_AI_PROVIDER=openai\nSYNTHPILOT_AI_MODEL=…\nSYNTHPILOT_AI_API_KEY=…");
    ui.add_space(4.0);
    ui.weak(
        "Use anthropic for Claude. SYNTHPILOT_AI_BASE_URL points to a local server such as Ollama.",
    );
}

fn prompt(ui: &mut egui::Ui, panel: &mut AssistantPanel, workspace: &Workspace, reading: bool) {
    let target = if workspace.part() == super::state::EFFECTS {
        "the Performance effects".to_owned()
    } else {
        format!("Part {} and the Performance effects", workspace.part())
    };
    ui.weak(format!("Changes {target}."));
    ui.add_space(4.0);
    let response = ui.add(
        egui::TextEdit::multiline(&mut panel.prompt)
            .hint_text("Describe the sound, e.g. a crystal Rhodes with a slow chorus")
            .desired_rows(3)
            .desired_width(f32::INFINITY),
    );
    let enter = response.has_focus()
        && ui.input(|input| input.key_pressed(egui::Key::Enter) && input.modifiers.command);
    ui.horizontal_wrapped(|ui| {
        for idea in IDEAS {
            if ui.small_button(idea).clicked() {
                idea.clone_into(&mut panel.prompt);
            }
        }
    });
    ui.add_space(4.0);
    let ready = !reading && !panel.is_busy() && !panel.prompt.trim().is_empty();
    let button = ui
        .add_enabled(
            ready,
            egui::Button::new(RichText::new("Propose changes").strong())
                .min_size(egui::vec2(ui.available_width(), 30.0)),
        )
        .on_hover_text(
            "Ask the AI for changes. Nothing is sent to the keyboard until you select Apply.",
        );
    if ready && (button.clicked() || enter) {
        panel.ask(workspace);
    }
}

fn plan(
    ui: &mut egui::Ui,
    panel: &mut AssistantPanel,
    workspace: &Workspace,
    actions: &mut Vec<Action>,
) {
    let Some(plan) = panel.plan() else {
        return;
    };
    let mut toggled = Vec::new();
    let chosen = plan.chosen();
    theme::scope_frame(ui, ACCENT, "Proposal", &plan.request, |ui| {
        if !plan.summary.is_empty() {
            ui.label(&plan.summary);
            ui.add_space(6.0);
        }
        if plan.rows.is_empty() {
            ui.weak("The AI didn't propose any changes.");
        }
        for (index, row) in plan.rows.iter().enumerate() {
            let resolved = &row.resolved;
            ui.horizontal_wrapped(|ui| {
                let mut selected = row.selected;
                let checkbox = ui.add_enabled(
                    resolved.is_applicable(),
                    egui::Checkbox::without_text(&mut selected),
                );
                if checkbox.changed() {
                    toggled.push(index);
                }
                let name = RichText::new(&resolved.name).strong();
                if resolved.is_applicable() {
                    ui.label(name);
                } else {
                    ui.label(name.weak());
                }
                if let (Some(from), Some(to)) = (&resolved.from, &resolved.to) {
                    ui.monospace(format!("{from} → {to}"));
                } else if let Some(to) = &resolved.to {
                    ui.monospace(format!("→ {to}"));
                }
            });
            if !resolved.reason.is_empty() {
                ui.weak(&resolved.reason);
            }
            if let Some(problem) = &resolved.problem {
                ui.label(RichText::new(problem).small().color(theme::WARNING));
            }
            ui.add_space(4.0);
        }
    });
    for index in toggled {
        panel.toggle(index);
    }
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let label = match chosen {
            1 => "Apply 1 change".to_owned(),
            count => format!("Apply {count} changes"),
        };
        if ui
            .add_enabled(chosen > 0, egui::Button::new(RichText::new(label).strong()))
            .on_hover_text("Send the selected changes to the keyboard.")
            .clicked()
        {
            actions.extend(panel.apply(workspace));
        }
        if ui.button("Discard").clicked() {
            panel.discard();
        }
    });
}
