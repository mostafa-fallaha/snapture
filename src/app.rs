use std::path::PathBuf;

use eframe::egui::{
    self, CentralPanel, Context, Key, KeyboardShortcut, Modifiers, SidePanel, TopBottomPanel,
};

use crate::{
    capture::CapturedImage,
    config::AppConfig,
    editor::{CanvasState, Document, HistoryManager, canvas},
    model::{
        overlay::{CropOverlay, OverlayObject},
        types::{ImagePoint, ImageRect, RgbaColor, StrokeStyle, TextStyle},
    },
    services::{clipboard, save},
    tools::{self, DraftOverlay, ToolKind},
    ui::{toolbar, topbar},
};

pub struct SnaptureApp {
    config: AppConfig,
    document: Document,
    history: HistoryManager,
    texture: Option<egui::TextureHandle>,
    texture_revision: u64,
    active_tool: ToolKind,
    draft: Option<DraftOverlay>,
    pending_crop: Option<ImageRect>,
    pending_text_anchor: Option<ImagePoint>,
    stroke_color: RgbaColor,
    stroke_thickness: f32,
    text_size: f32,
    text_buffer: String,
    save_path: String,
    canvas_state: CanvasState,
    status: String,
}

impl SnaptureApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        config: AppConfig,
        initial_capture: CapturedImage,
    ) -> Self {
        let document = Document::from_image(initial_capture.image, initial_capture.source_uri);
        let history = HistoryManager::new(config.history_limit);
        let save_path = config.default_save_path().display().to_string();

        let mut app = Self {
            config,
            document,
            history,
            texture: None,
            texture_revision: 0,
            active_tool: ToolKind::Pen,
            draft: None,
            pending_crop: None,
            pending_text_anchor: None,
            stroke_color: RgbaColor::default(),
            stroke_thickness: 4.0,
            text_size: 28.0,
            text_buffer: String::new(),
            save_path,
            canvas_state: CanvasState::default(),
            status: "Screenshot captured. Draw, crop, or add text.".into(),
        };

        app.stroke_color = app.config.default_color;
        app.stroke_thickness = app.config.default_stroke_thickness;
        app.text_size = app.config.default_text_size;
        app.refresh_texture(&cc.egui_ctx);
        app
    }

    fn refresh_texture(&mut self, ctx: &Context) {
        let image = self.document.base_image();
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_raw(),
        );
        let name = format!("document-base-{}", self.texture_revision);
        self.texture = Some(ctx.load_texture(name, color_image, egui::TextureOptions::LINEAR));
        self.texture_revision += 1;
    }

    fn current_stroke_style(&self) -> StrokeStyle {
        StrokeStyle::new(self.stroke_color, self.stroke_thickness)
    }

    fn current_text_style(&self) -> TextStyle {
        TextStyle::new(self.stroke_color, self.text_size)
    }

    fn set_status(&mut self, message: impl Into<String>) {
        self.status = message.into();
    }

    fn clear_transient_state(&mut self) {
        self.draft = None;
        self.pending_crop = None;
        self.pending_text_anchor = None;
    }

    fn set_active_tool(&mut self, tool: ToolKind) {
        if self.active_tool != tool {
            self.active_tool = tool;
            self.clear_transient_state();
        }
    }

    fn preview_overlays(&self) -> Vec<OverlayObject> {
        let mut overlays = Vec::new();

        if let Some(draft) = &self.draft {
            overlays.push(draft.preview());
        }

        if let Some(crop) = self.pending_crop {
            overlays.push(OverlayObject::Crop(CropOverlay { rect: crop }));
        }

        if let Some(anchor) = self.pending_text_anchor {
            if let Some(overlay) = tools::text::build_text_overlay(
                anchor,
                self.text_buffer.clone(),
                self.current_text_style(),
            ) {
                overlays.push(overlay);
            }
        }

        overlays
    }

    fn commit_overlay(&mut self, overlay: OverlayObject) {
        self.history.checkpoint(&self.document);
        self.document.add_overlay(overlay);
        self.set_status("Annotation added.");
    }

    fn commit_crop(&mut self, ctx: &Context) {
        let Some(selection) = self.pending_crop.take() else {
            return;
        };

        self.history.checkpoint(&self.document);
        match self.document.crop_to(selection) {
            Ok(()) => {
                self.refresh_texture(ctx);
                self.set_status("Crop applied.");
            }
            Err(error) => self.set_status(format!("Crop failed: {error}")),
        }
    }

    fn undo(&mut self, ctx: &Context) {
        if self.history.undo(&mut self.document) {
            self.clear_transient_state();
            self.refresh_texture(ctx);
            self.set_status("Undid last action.");
        }
    }

    fn redo(&mut self, ctx: &Context) {
        if self.history.redo(&mut self.document) {
            self.clear_transient_state();
            self.refresh_texture(ctx);
            self.set_status("Redid last action.");
        }
    }

    fn save_document(&mut self) {
        let path = match save::choose_save_path(PathBuf::from(&self.save_path)) {
            Ok(Some(path)) => path,
            Ok(None) => {
                self.set_status("Save cancelled.");
                return;
            }
            Err(error) => {
                self.set_status(format!("Save dialog failed: {error}"));
                return;
            }
        };

        match save::save_document_png(&self.document, &path) {
            Ok(path) => {
                self.save_path = path.display().to_string();
                self.set_status(format!("Saved {}", path.display()));
            }
            Err(error) => self.set_status(format!("Save failed: {error}")),
        }
    }

    fn copy_document(&mut self) {
        match clipboard::copy_document_image(&self.document) {
            Ok(()) => self.set_status("Copied image to clipboard."),
            Err(error) => self.set_status(format!("Clipboard failed: {error}")),
        }
    }

    fn show_text_editor(&mut self, ctx: &Context) {
        let Some(anchor) = self.pending_text_anchor else {
            return;
        };

        egui::Window::new("Text Annotation")
            .collapsible(false)
            .resizable(false)
            .default_width(320.0)
            .anchor(egui::Align2::RIGHT_TOP, [-16.0, 72.0])
            .show(ctx, |ui| {
                ui.label(format!("Anchor: {:.0}, {:.0}", anchor.x, anchor.y));
                ui.add(
                    egui::TextEdit::multiline(&mut self.text_buffer)
                        .desired_rows(4)
                        .desired_width(280.0)
                        .hint_text("Type text and press Add Text"),
                );

                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !self.text_buffer.trim().is_empty(),
                            egui::Button::new("Add Text"),
                        )
                        .clicked()
                    {
                        if let Some(overlay) = tools::text::build_text_overlay(
                            anchor,
                            self.text_buffer.clone(),
                            self.current_text_style(),
                        ) {
                            self.commit_overlay(overlay);
                            self.pending_text_anchor = None;
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        self.pending_text_anchor = None;
                    }
                });
            });
    }

    fn handle_canvas_output(&mut self, output: canvas::CanvasOutput, ctx: &Context) {
        match self.active_tool {
            ToolKind::Pen | ToolKind::Rectangle | ToolKind::Arrow | ToolKind::Crop => {
                if let Some(start) = output.drag_started {
                    self.draft =
                        tools::begin_drag(self.active_tool, start, self.current_stroke_style());
                    if self.active_tool == ToolKind::Crop {
                        self.pending_crop = None;
                    }
                }

                if let Some(current) = output.drag_current {
                    if let Some(draft) = &mut self.draft {
                        draft.update(current);
                    }
                }

                if let Some(end) = output.drag_stopped {
                    if let Some(mut draft) = self.draft.take() {
                        draft.update(end);
                        match draft.finish() {
                            Some(OverlayObject::Crop(crop)) => {
                                self.pending_crop = Some(crop.rect);
                                self.set_status(
                                    "Crop region selected. Commit or cancel it in the toolbar.",
                                );
                            }
                            Some(overlay) => self.commit_overlay(overlay),
                            None => {}
                        }
                    }
                    ctx.request_repaint();
                }
            }
            ToolKind::Text => {
                if let Some(position) = output.clicked {
                    self.pending_text_anchor = Some(position);
                    self.set_status("Text anchor placed. Finish the text in the floating editor.");
                }
            }
        }
    }

    fn handle_shortcuts(&mut self, ctx: &Context) {
        let undo_shortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::Z);
        let redo_shortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::Y);
        let redo_alt_shortcut = KeyboardShortcut::new(Modifiers::CTRL | Modifiers::SHIFT, Key::Z);
        let save_shortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::S);
        let copy_shortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::C);

        if ctx.input_mut(|input| input.consume_shortcut(&undo_shortcut)) {
            self.undo(ctx);
        }
        if ctx.input_mut(|input| {
            input.consume_shortcut(&redo_shortcut) || input.consume_shortcut(&redo_alt_shortcut)
        }) {
            self.redo(ctx);
        }
        if ctx.input_mut(|input| input.consume_shortcut(&save_shortcut)) {
            self.save_document();
        }
        if ctx.input_mut(|input| input.consume_shortcut(&copy_shortcut)) {
            self.copy_document();
        }
    }
}

impl eframe::App for SnaptureApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);

        TopBottomPanel::top("topbar").show(ctx, |ui| {
            let output = topbar::show(
                ui,
                self.history.can_undo(),
                self.history.can_redo(),
                &self.status,
            );

            if output.save_clicked {
                self.save_document();
            }
            if output.copy_clicked {
                self.copy_document();
            }
            if output.undo_clicked {
                self.undo(ctx);
            }
            if output.redo_clicked {
                self.redo(ctx);
            }
            if output.fit_clicked {
                self.canvas_state.zoom = 1.0;
            }
        });

        SidePanel::left("toolbar")
            .resizable(false)
            .default_width(260.0)
            .show(ctx, |ui| {
                let output = toolbar::show(
                    ui,
                    self.active_tool,
                    &mut self.stroke_color,
                    &mut self.stroke_thickness,
                    &mut self.text_size,
                    &mut self.text_buffer,
                    &mut self.save_path,
                    &mut self.canvas_state.zoom,
                    self.config.min_zoom,
                    self.config.max_zoom,
                    self.pending_crop.is_some(),
                    self.pending_text_anchor.is_some(),
                );

                if let Some(tool) = output.tool_change {
                    self.set_active_tool(tool);
                }
                if output.commit_crop {
                    self.commit_crop(ctx);
                }
                if output.cancel_crop {
                    self.pending_crop = None;
                    self.set_status("Crop cancelled.");
                }
            });

        CentralPanel::default().show(ctx, |ui| {
            let preview_overlays = self.preview_overlays();
            let output = canvas::show(
                ui,
                &self.document,
                self.texture.as_ref(),
                &mut self.canvas_state,
                &preview_overlays,
            );
            self.handle_canvas_output(output, ctx);
        });

        self.show_text_editor(ctx);
    }
}
