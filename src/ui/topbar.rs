use eframe::egui;

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
    status: &str,
) -> TopbarOutput {
    let mut output = TopbarOutput::default();

    ui.horizontal_wrapped(|ui| {
        if ui.button("Save PNG... (Ctrl+S)").clicked() {
            output.save_clicked = true;
        }

        if ui.button("Copy Image (Ctrl+C)").clicked() {
            output.copy_clicked = true;
        }

        if ui
            .add_enabled(
                !extracting_text,
                egui::Button::new(if extracting_text {
                    "Extracting Text..."
                } else {
                    "Extract Text"
                }),
            )
            .clicked()
        {
            output.extract_text_clicked = true;
        }

        ui.separator();

        if ui
            .add_enabled(can_undo, egui::Button::new("Undo"))
            .clicked()
        {
            output.undo_clicked = true;
        }

        if ui
            .add_enabled(can_redo, egui::Button::new("Redo"))
            .clicked()
        {
            output.redo_clicked = true;
        }

        if ui.button("Fit").clicked() {
            output.fit_clicked = true;
        }

        ui.separator();
        ui.label(status);
    });

    output
}
