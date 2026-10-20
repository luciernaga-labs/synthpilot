use std::time::{Duration, Instant};

use eframe::egui::{
    self, Color32, CornerRadius, FontFamily, FontId, Margin, RichText, Stroke, TextStyle,
};
use synthpilot_midi::Port;

pub(crate) const SUCCESS: Color32 = Color32::from_rgb(0x3F, 0xB9, 0x50);
pub(crate) const IDLE: Color32 = Color32::from_rgb(0x9A, 0x9A, 0x9A);
pub(crate) const ACTIVITY: Color32 = Color32::from_rgb(0xF0, 0x88, 0x3E);
pub(crate) const WARNING: Color32 = Color32::from_rgb(0xE3, 0xB3, 0x41);
pub(crate) const ERROR: Color32 = Color32::from_rgb(0xF0, 0x6A, 0x62);
pub(crate) const PART_ACCENT: Color32 = Color32::from_rgb(0x5A, 0xA9, 0xE6);
pub(crate) const ELEMENT_ACCENT: Color32 = Color32::from_rgb(0xA9, 0x8B, 0xEA);
pub(crate) const TEXT: Color32 = Color32::from_rgb(0xDA, 0xDA, 0xDA);
pub(crate) const WEAK_TEXT: Color32 = Color32::from_rgb(0xA8, 0xA8, 0xA8);

const AWM2_COLOR: Color32 = Color32::from_rgb(0x4F, 0xB0, 0xA8);
const FM_X_COLOR: Color32 = Color32::from_rgb(0xE0, 0x8A, 0x5A);
const DRUM_COLOR: Color32 = Color32::from_rgb(0xC9, 0x7B, 0xC0);

const LED_TIME: Duration = Duration::from_millis(180);

pub(crate) fn apply(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (
                TextStyle::Heading,
                FontId::new(21.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
            (
                TextStyle::Button,
                FontId::new(14.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Small,
                FontId::new(11.5, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(13.0, FontFamily::Monospace),
            ),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        style.spacing.interact_size.y = 24.0;
        style.spacing.slider_width = 220.0;
        style.visuals.weak_text_color = Some(WEAK_TEXT);
        style.visuals.widgets.noninteractive.fg_stroke.color = TEXT;
        style.visuals.widgets.inactive.fg_stroke.color = TEXT;
        style.visuals.selection.stroke = Stroke::new(2.0, Color32::WHITE);
        style.visuals.widgets.hovered.bg_stroke.width = 1.5;
    });
}

pub(crate) fn status_badge(ui: &mut egui::Ui, live_port: Option<&Port>, sent: usize) {
    let (text, hover, color) = match live_port {
        Some(port) => (
            format!("Connected · {port}"),
            "Every change you make is sent to the keyboard right away. Store the Performance on the MODX+ to keep it.",
            SUCCESS,
        ),
        None => (
            "Not connected".to_owned(),
            "Choose your keyboard's MIDI port, then select Connect and read Performance.",
            IDLE,
        ),
    };
    if live_port.is_some() {
        midi_led(ui, sent);
    }
    ui.label(RichText::new(text).color(color))
        .on_hover_text(hover);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 5.0, color);
}

fn midi_led(ui: &mut egui::Ui, sent: usize) {
    let id = egui::Id::unique("midi_led");
    let now = Instant::now();
    let last = ui.data(|data| data.get_temp::<(usize, Option<Instant>)>(id));
    let flashed = match last {
        Some((count, at)) if count == sent => at,
        Some(_) => Some(now),
        None => None,
    };
    ui.data_mut(|data| data.insert_temp(id, (sent, flashed)));
    let lit = flashed.is_some_and(|at| now.duration_since(at) < LED_TIME);
    if lit {
        ui.ctx().request_repaint_after(LED_TIME);
    }
    let color = if lit {
        ACTIVITY
    } else {
        ACTIVITY.gamma_multiply(0.25)
    };
    ui.label(RichText::new("MIDI").small().color(color))
        .on_hover_text(format!("{sent} MIDI messages sent to the keyboard."));
    let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
    ui.add_space(10.0);
}

pub(crate) fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(12.0);
    heading(ui, title);
    ui.add_space(2.0);
}

pub(crate) fn heading(ui: &mut egui::Ui, title: &str) {
    ui.label(
        RichText::new(title.to_uppercase())
            .small()
            .strong()
            .color(WEAK_TEXT),
    );
}

pub(crate) fn engine_color(engine: &str) -> Color32 {
    match engine {
        "AWM2" => AWM2_COLOR,
        "FM-X" => FM_X_COLOR,
        "Drum" => DRUM_COLOR,
        _ => IDLE,
    }
}

pub(crate) fn engine_chip(ui: &mut egui::Ui, engine: &str) {
    let color = engine_color(engine);
    egui::Frame::new()
        .fill(color.gamma_multiply(0.18))
        .stroke(Stroke::new(1.0, color))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::symmetric(6, 1))
        .show(ui, |ui| {
            ui.label(RichText::new(engine).small().strong().color(color));
        });
}

pub(crate) fn scope_frame<R>(
    ui: &mut egui::Ui,
    accent: Color32,
    title: &str,
    subtitle: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let inner = egui::Frame::new()
        .fill(accent.gamma_multiply(0.07))
        .stroke(Stroke::new(1.0, accent.gamma_multiply(0.35)))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin {
            left: 18,
            right: 14,
            top: 10,
            bottom: 12,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(title).strong().size(16.0).color(accent));
                ui.label(RichText::new(subtitle).weak());
            });
            ui.add_space(4.0);
            add_contents(ui)
        });
    let rect = inner.response.rect;
    ui.painter().rect_filled(
        egui::Rect::from_min_max(rect.min, egui::pos2(rect.min.x + 5.0, rect.max.y)),
        CornerRadius {
            nw: 6,
            sw: 6,
            ne: 0,
            se: 0,
        },
        accent,
    );
    inner.inner
}

pub(crate) fn banner(
    ui: &mut egui::Ui,
    color: Color32,
    text: &str,
    add_actions: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.14))
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.6)))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(text).color(color));
                add_actions(ui);
            });
        });
}

pub(crate) fn errors(ui: &mut egui::Ui, errors: &[&str]) {
    let id = egui::Id::unique("dismissed_errors");
    let mut dismissed: Vec<String> = ui.data(|data| data.get_temp(id)).unwrap_or_default();
    dismissed.retain(|text| errors.contains(&text.as_str()));
    for error in errors {
        if dismissed.iter().any(|text| text == error) {
            continue;
        }
        banner(ui, ERROR, error, |ui| {
            if ui
                .small_button("Hide")
                .on_hover_text("Hide this message")
                .clicked()
            {
                dismissed.push((*error).to_owned());
            }
        });
        ui.add_space(6.0);
    }
    ui.data_mut(|data| data.insert_temp(id, dismissed));
}
