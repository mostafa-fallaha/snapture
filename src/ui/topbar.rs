use eframe::egui::{self, Button, CornerRadius, Frame, Layout, Margin, RichText, Stroke, vec2};

use crate::ui::theme;

#[derive(Default)]
pub struct TopbarOutput {
    pub save_clicked: bool,
    pub copy_clicked: bool,
    pub extract_text_clicked: bool,
    pub undo_clicked: bool,
    pub redo_clicked: bool,
    pub fit_clicked: bool,
}

pub fn show(
    ui: &mut egui::Ui,
    can_undo: bool,
    can_redo: bool,
    extracting_text: bool,
    saving_document: bool,
    status: &str,
) -> TopbarOutput {
    let mut output = TopbarOutput::default();

    ui.horizontal(|ui| {
        action_group(ui, |ui| {
            let save = ui.add_enabled(
                !saving_document,
                Button::new(
                    RichText::new(if saving_document { "Saving..." } else { "Save" })
                        .size(12.5)
                        .strong(),
                )
                .fill(theme::ACCENT)
                .stroke(Stroke::new(1.0, theme::ACCENT_HOVER))
                .corner_radius(CornerRadius::same(8))
                .min_size(vec2(74.0, theme::CONTROL_HEIGHT)),
            );
            if save.clicked() {
                output.save_clicked = true;
            }
            save.on_hover_text("Save PNG (Ctrl+S)");

            let copy = ui.add(
                Button::new(RichText::new("Copy").size(12.5))
                    .corner_radius(CornerRadius::same(8))
                    .min_size(vec2(70.0, theme::CONTROL_HEIGHT)),
            );
            if copy.clicked() {
                output.copy_clicked = true;
            }
            copy.on_hover_text("Copy image (Ctrl+C)");

            let extract = ui.add_enabled(
                !extracting_text,
                Button::new(
                    RichText::new(if extracting_text {
                        "Extracting..."
                    } else {
                        "Extract Text"
                    })
                    .size(12.5),
                )
                .corner_radius(CornerRadius::same(8))
                .min_size(vec2(108.0, theme::CONTROL_HEIGHT)),
            );
            if extract.clicked() {
                output.extract_text_clicked = true;
            }
        });

        action_group(ui, |ui| {
            let undo = ui.add_enabled(
                can_undo,
                Button::new(RichText::new("Undo").size(12.5))
                    .corner_radius(CornerRadius::same(8))
                    .min_size(vec2(64.0, theme::CONTROL_HEIGHT)),
            );
            if undo.clicked() {
                output.undo_clicked = true;
            }
            undo.on_hover_text("Undo (Ctrl+Z)");

            let redo = ui.add_enabled(
                can_redo,
                Button::new(RichText::new("Redo").size(12.5))
                    .corner_radius(CornerRadius::same(8))
                    .min_size(vec2(64.0, theme::CONTROL_HEIGHT)),
            );
            if redo.clicked() {
                output.redo_clicked = true;
            }
            redo.on_hover_text("Redo (Ctrl+Y)");

            let fit = ui.add(
                Button::new(RichText::new("Fit").size(12.5))
                    .corner_radius(CornerRadius::same(8))
                    .min_size(vec2(54.0, theme::CONTROL_HEIGHT)),
            );
            if fit.clicked() {
                output.fit_clicked = true;
            }
            fit.on_hover_text("Reset zoom to fit");
        });

        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            status_badge(ui, status, extracting_text || saving_document);
        });
    });

    output
}

fn action_group(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    Frame::new()
        .fill(theme::SECTION_BG)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(6, 4))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 0.0);
            ui.horizontal(|ui| add_contents(ui));
        });
}

fn status_badge(ui: &mut egui::Ui, status: &str, active: bool) {
    let fill = if active {
        theme::ACCENT_ACTIVE
    } else {
        theme::SECTION_BG
    };
    let stroke = if active {
        Stroke::new(1.0, theme::ACCENT_HOVER)
    } else {
        Stroke::new(1.0, theme::BORDER)
    };

    Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(RichText::new(status).size(11.5).color(if active {
                    theme::TEXT
                } else {
                    theme::TEXT_MUTED
                }))
                .wrap(),
            );
        });
}
