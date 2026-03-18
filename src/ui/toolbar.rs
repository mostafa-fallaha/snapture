use eframe::egui::{self, color_picker};

use crate::{model::types::RgbaColor, tools::ToolKind};

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

    ui.heading("Tools");
    ui.separator();

    for tool in ToolKind::ALL {
        if ui
            .selectable_label(active_tool == tool, tool.label())
            .clicked()
        {
            output.tool_change = Some(tool);
        }
    }

    ui.separator();
    ui.label("Stroke");

    let mut egui_color = color.to_egui();
    if color_picker::color_edit_button_srgba(ui, &mut egui_color, color_picker::Alpha::Opaque)
        .changed()
    {
        *color = RgbaColor::from(egui_color);
    }

    let (active_thickness, thickness_range) = if active_tool == ToolKind::Highlighter {
        (highlighter_thickness, 10.0..=34.0)
    } else {
        (stroke_thickness, 1.0..=24.0)
    };

    ui.add(
        egui::Slider::new(active_thickness, thickness_range)
            .text("Thickness")
            .clamping(egui::SliderClamping::Always),
    );

    if active_tool == ToolKind::Highlighter {
        let mut transparency = 100.0 - (f32::from(*highlighter_alpha) / 255.0) * 100.0;
        if ui
            .add(
                egui::Slider::new(&mut transparency, 0.0..=100.0)
                    .text("Transparency")
                    .clamping(egui::SliderClamping::Always),
            )
            .changed()
        {
            *highlighter_alpha = ((100.0 - transparency) / 100.0 * 255.0).round() as u8;
        }
    }

    ui.separator();
    ui.label("Text");
    ui.add(
        egui::Slider::new(text_size, 10.0..=96.0)
            .text("Size")
            .clamping(egui::SliderClamping::Always),
    );

    if active_tool == ToolKind::Select && has_selected_overlay {
        ui.separator();
        if ui.button("Delete Selected").clicked() {
            output.delete_selected = true;
        }
    }

    if has_pending_crop {
        ui.separator();
        ui.label("Crop");

        if ui.button("Commit Crop (Enter)").clicked() {
            output.commit_crop = true;
        }

        if ui.button("Cancel Crop (Esc)").clicked() {
            output.cancel_crop = true;
        }
    }

    ui.separator();
    ui.label("View");
    ui.add(
        egui::Slider::new(zoom, min_zoom..=max_zoom)
            .text("Zoom")
            .clamping(egui::SliderClamping::Always),
    );

    ui.separator();
    ui.label("Save Path");
    ui.add(egui::TextEdit::singleline(save_path).desired_width(f32::INFINITY));

    output
}
