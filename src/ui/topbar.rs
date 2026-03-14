use eframe::egui;

#[derive(Default)]
pub struct TopbarOutput {
    pub capture_clicked: bool,
    pub save_clicked: bool,
    pub copy_clicked: bool,
    pub undo_clicked: bool,
    pub redo_clicked: bool,
    pub fit_clicked: bool,
}

pub fn show(
    ui: &mut egui::Ui,
    can_undo: bool,
    can_redo: bool,
    capture_in_progress: bool,
    status: &str,
) -> TopbarOutput {
    let mut output = TopbarOutput::default();

    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(
                !capture_in_progress,
                egui::Button::new("Capture Screenshot"),
            )
            .clicked()
        {
            output.capture_clicked = true;
        }

        if ui.button("Save PNG").clicked() {
            output.save_clicked = true;
        }

        if ui.button("Copy Image").clicked() {
            output.copy_clicked = true;
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
