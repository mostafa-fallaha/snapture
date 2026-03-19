use std::ops::RangeInclusive;

use eframe::egui::{self, Button, Color32, CornerRadius, RichText, Stroke, color_picker, vec2};

use crate::{model::types::RgbaColor, tools::ToolKind, ui::theme};

const SWATCH_SIZE: f32 = 22.0;
const CURATED_COLORS: [(&str, RgbaColor); 7] = [
    ("Red", RgbaColor::from_rgba(255, 59, 48, 255)),
    ("Orange", RgbaColor::from_rgba(255, 149, 0, 255)),
    ("Yellow", RgbaColor::from_rgba(255, 214, 10, 255)),
    ("Green", RgbaColor::from_rgba(52, 199, 89, 255)),
    ("Cyan", RgbaColor::from_rgba(90, 200, 250, 255)),
    ("Blue", RgbaColor::from_rgba(10, 132, 255, 255)),
    ("White", RgbaColor::from_rgba(245, 247, 250, 255)),
];

#[derive(Default)]
pub struct ToolbarOutput {
    pub tool_change: Option<ToolKind>,
    pub delete_selected: bool,
    pub commit_crop: bool,
    pub cancel_crop: bool,
}

pub fn show(
    ui: &mut egui::Ui,
    active_tool: ToolKind,
    color: &mut RgbaColor,
    stroke_thickness: &mut f32,
    highlighter_thickness: &mut f32,
    highlighter_alpha: &mut u8,
    text_size: &mut f32,
    save_path: &mut String,
    zoom: &mut f32,
    min_zoom: f32,
    max_zoom: f32,
    has_selected_overlay: bool,
    has_pending_crop: bool,
) -> ToolbarOutput {
    let mut output = ToolbarOutput::default();

    ui.spacing_mut().item_spacing.y = 10.0;

    section(ui, "Tools", |ui| {
        ui.style_mut().spacing.item_spacing = vec2(8.0, 8.0);
        let tool_spacing = 8.0;
        let button_width = ((ui.available_width() - tool_spacing) * 0.5).max(80.0);
        egui::Grid::new("tool_grid")
            .num_columns(2)
            .spacing(vec2(tool_spacing, 8.0))
            .show(ui, |ui| {
                for chunk in ToolKind::ALL.chunks(2) {
                    for tool in chunk {
                        let response = ui.add(
                            Button::new(
                                RichText::new(tool.label())
                                    .size(12.5)
                                    .strong()
                                    .color(theme::TEXT),
                            )
                            .selected(active_tool == *tool)
                            .corner_radius(CornerRadius::same(8))
                            .min_size(vec2(button_width, theme::CONTROL_HEIGHT)),
                        );

                        if response.clicked() {
                            output.tool_change = Some(*tool);
                        }
                    }
                    ui.end_row();
                }
            });
    });

    section(ui, "Appearance", |ui| {
        palette_row(ui, color);

        let (active_thickness, thickness_range) = if active_tool == ToolKind::Highlighter {
            (highlighter_thickness, 10.0..=34.0)
        } else {
            (stroke_thickness, 1.0..=24.0)
        };

        slider_row(
            ui,
            "Thickness",
            active_thickness,
            thickness_range,
            |value| format!("{value:.0}px"),
        );

        if active_tool == ToolKind::Highlighter {
            let mut opacity = f32::from(*highlighter_alpha) / 255.0 * 100.0;
            if slider_row(ui, "Opacity", &mut opacity, 0.0..=100.0, |value| {
                format!("{value:.0}%")
            })
            .changed()
            {
                *highlighter_alpha = (opacity / 100.0 * 255.0).round() as u8;
            }
        }
    });

    section(ui, "Text", |ui| {
        slider_row(ui, "Size", text_size, 10.0..=96.0, |value| {
            format!("{value:.0}pt")
        });
    });

    if active_tool == ToolKind::Select && has_selected_overlay {
        section(ui, "Selection", |ui| {
            let response = ui.add(
                Button::new(
                    RichText::new("Delete selected")
                        .size(12.5)
                        .strong()
                        .color(theme::TEXT),
                )
                .fill(theme::DANGER)
                .stroke(Stroke::new(1.0, Color32::from_rgb(226, 114, 114)))
                .corner_radius(CornerRadius::same(8))
                .min_size(vec2(ui.available_width(), theme::CONTROL_HEIGHT)),
            );

            if response.clicked() {
                output.delete_selected = true;
            }
        });
    }

    if has_pending_crop {
        section(ui, "Crop", |ui| {
            ui.label(
                RichText::new("Resize the crop box, then commit or cancel.")
                    .size(11.5)
                    .color(theme::TEXT_MUTED),
            );

            ui.horizontal(|ui| {
                let commit = ui.add(
                    Button::new(RichText::new("Commit").size(12.5).strong())
                        .fill(theme::SUCCESS)
                        .stroke(Stroke::new(1.0, Color32::from_rgb(112, 204, 152)))
                        .corner_radius(CornerRadius::same(8))
                        .min_size(vec2(88.0, theme::CONTROL_HEIGHT)),
                );
                if commit.clicked() {
                    output.commit_crop = true;
                }
                commit.on_hover_text("Apply crop (Enter)");

                let cancel = ui.add(
                    Button::new(RichText::new("Cancel").size(12.5).color(theme::TEXT))
                        .corner_radius(CornerRadius::same(8))
                        .min_size(vec2(88.0, theme::CONTROL_HEIGHT)),
                );
                if cancel.clicked() {
                    output.cancel_crop = true;
                }
                cancel.on_hover_text("Cancel crop (Esc)");
            });
        });
    }

    section(ui, "View", |ui| {
        slider_row(ui, "Zoom", zoom, min_zoom..=max_zoom, |value| {
            format!("{:.0}%", value * 100.0)
        });
    });

    section(ui, "Export", |ui| {
        ui.label(
            RichText::new("Suggested save path")
                .size(11.5)
                .color(theme::TEXT_MUTED),
        );
        ui.add(
            egui::TextEdit::singleline(save_path)
                .desired_width(f32::INFINITY)
                .hint_text("Choose a save location"),
        );
    });

    output
}

fn section(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    theme::section_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(
            RichText::new(title)
                .size(12.0)
                .strong()
                .color(theme::TEXT_MUTED),
        );
        ui.add_space(8.0);
        add_contents(ui);
    });
}

fn palette_row(ui: &mut egui::Ui, color: &mut RgbaColor) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);

        for (name, swatch) in CURATED_COLORS {
            let selected = *color == swatch;
            let stroke = if selected {
                Stroke::new(2.0, theme::ACCENT_HOVER)
            } else {
                Stroke::new(1.0, theme::BORDER)
            };

            let response = ui.add(
                Button::new("")
                    .fill(swatch.to_egui())
                    .stroke(stroke)
                    .corner_radius(CornerRadius::same(6))
                    .min_size(vec2(SWATCH_SIZE, SWATCH_SIZE)),
            );

            if response.clicked() {
                *color = swatch;
            }

            response.on_hover_text(name);
        }

        ui.add_space(4.0);
        ui.label(RichText::new("Custom").size(11.0).color(theme::TEXT_MUTED));

        let mut egui_color = color.to_egui();
        if color_picker::color_edit_button_srgba(ui, &mut egui_color, color_picker::Alpha::Opaque)
            .changed()
        {
            *color = RgbaColor::from(egui_color);
        }
    });
}

fn slider_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: RangeInclusive<f32>,
    format_value: impl Fn(f32) -> String,
) -> egui::Response {
    ui.horizontal(|ui| {
        ui.add_sized(
            [64.0, theme::CONTROL_HEIGHT],
            egui::Label::new(RichText::new(label).size(12.0).color(theme::TEXT_MUTED)),
        );

        let slider_width = (ui.available_width() - 50.0).max(72.0);
        let response = ui.add_sized(
            [slider_width, theme::CONTROL_HEIGHT],
            egui::Slider::new(value, range)
                .show_value(false)
                .clamping(egui::SliderClamping::Always),
        );

        ui.add_sized(
            [46.0, theme::CONTROL_HEIGHT],
            egui::Label::new(
                RichText::new(format_value(*value))
                    .monospace()
                    .size(11.5)
                    .color(theme::TEXT_MUTED),
            ),
        );

        response
    })
    .inner
}
