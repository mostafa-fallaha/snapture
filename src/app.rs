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
    text_editor_should_focus: bool,
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
            text_editor_should_focus: false,
            stroke_color: RgbaColor::default(),
            stroke_thickness: 4.0,
            text_size: 28.0,
            text_buffer: String::new(),
            save_path,
            canvas_state: CanvasState::default(),
            status: format!(
                "Screenshot captured. {}",
                Self::tool_status_message(ToolKind::Pen)
            ),
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
        self.text_editor_should_focus = false;
    }

    fn full_image_crop_rect(&self) -> ImageRect {
        let [width, height] = self.document.image_size();
        ImageRect::from_points(
            ImagePoint::new(0.0, 0.0),
            ImagePoint::new(width as f32, height as f32),
        )
    }

    fn activate_tool(&mut self, tool: ToolKind) {
        if self.active_tool != tool {
            self.active_tool = tool;
            self.clear_transient_state();
        }

        if tool == ToolKind::Crop {
            self.draft = None;
            self.pending_text_anchor = None;
            self.pending_crop = Some(self.full_image_crop_rect());
        }

        self.set_status(Self::tool_status_message(tool));
    }

    fn tool_status_message(tool: ToolKind) -> &'static str {
        match tool {
            ToolKind::Pen => "Pen active. Drag on the image to draw.",
            ToolKind::Rectangle => "Rectangle active. Drag on the image to place a box.",
            ToolKind::Arrow => "Arrow active. Drag on the image to place an arrow.",
            ToolKind::Text => {
                "Text active. Click the image to place a text anchor and type in the floating dialog, or drag existing text to reposition it."
            }
            ToolKind::Crop => {
                "Crop active. Resize or move the crop box, then commit or cancel it in the toolbar."
            }
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

    fn commit_overlay(&mut self, overlay: OverlayObject, status: &'static str) {
        self.history.checkpoint(&self.document);
        self.document.add_overlay(overlay);
        self.set_status(status);
    }

    fn commit_crop(&mut self, ctx: &Context) {
        let Some(selection) = self.pending_crop.take() else {
            return;
        };

        self.history.checkpoint(&self.document);
        match self.document.crop_to(selection) {
            Ok(()) => {
                self.refresh_texture(ctx);
                self.set_status(
                    "Crop applied. Click Crop again if you want to start another crop.",
                );
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
        let should_focus = self.text_editor_should_focus;

        egui::Window::new("Text Annotation")
            .collapsible(false)
            .resizable(false)
            .default_width(320.0)
            .anchor(egui::Align2::RIGHT_TOP, [-16.0, 72.0])
            .show(ctx, |ui| {
                ui.label(format!("Anchor: {:.0}, {:.0}", anchor.x, anchor.y));
                let response = ui.add(
                    egui::TextEdit::multiline(&mut self.text_buffer)
                        .desired_rows(4)
                        .desired_width(280.0)
                        .hint_text("Type text and press Add Text"),
                );
                if should_focus {
                    response.request_focus();
                }

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
                            self.commit_overlay(
                                overlay,
                                "Text added. Click the image to place another text annotation, or drag existing text to reposition it.",
                            );
                            self.pending_text_anchor = None;
                            self.text_editor_should_focus = false;
                            self.text_buffer.clear();
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        self.pending_text_anchor = None;
                        self.text_editor_should_focus = false;
                        self.text_buffer.clear();
                        self.set_status(Self::tool_status_message(ToolKind::Text));
                    }
                });
            });

        if should_focus {
            self.text_editor_should_focus = false;
        }
    }

    fn handle_canvas_output(&mut self, output: canvas::CanvasOutput, ctx: &Context) {
        match self.active_tool {
            ToolKind::Pen => self.handle_pen_output(&output, ctx),
            ToolKind::Rectangle => self.handle_rectangle_output(&output, ctx),
            ToolKind::Arrow => self.handle_arrow_output(&output, ctx),
            ToolKind::Text => self.handle_text_output(&output),
            ToolKind::Crop => self.handle_crop_output(&output),
        }
    }

    fn handle_pen_output(&mut self, output: &canvas::CanvasOutput, ctx: &Context) {
        self.handle_shape_output(
            output,
            ctx,
            ToolKind::Pen,
            "Pen stroke added. Drag again to keep drawing.",
        );
    }

    fn handle_rectangle_output(&mut self, output: &canvas::CanvasOutput, ctx: &Context) {
        self.handle_shape_output(
            output,
            ctx,
            ToolKind::Rectangle,
            "Rectangle added. Drag again to place another box.",
        );
    }

    fn handle_arrow_output(&mut self, output: &canvas::CanvasOutput, ctx: &Context) {
        self.handle_shape_output(
            output,
            ctx,
            ToolKind::Arrow,
            "Arrow added. Drag again to place another arrow.",
        );
    }

    fn handle_shape_output(
        &mut self,
        output: &canvas::CanvasOutput,
        ctx: &Context,
        tool: ToolKind,
        commit_status: &'static str,
    ) {
        if let Some(start) = output.drag_started {
            self.draft = tools::begin_drag(tool, start, self.current_stroke_style());
        }

        if let Some(current) = output.drag_current {
            if let Some(draft) = &mut self.draft {
                draft.update(current);
            }
        }

        if let Some(end) = output.drag_stopped {
            if let Some(mut draft) = self.draft.take() {
                draft.update(end);
                if let Some(overlay) = draft.finish() {
                    self.commit_overlay(overlay, commit_status);
                }
            }
            ctx.request_repaint();
        }
    }

    fn handle_crop_output(&mut self, output: &canvas::CanvasOutput) {
        if let Some(crop_rect) = output.crop_rect {
            self.pending_crop = Some(crop_rect);
            self.set_status(
                "Crop box updated. Commit it or cancel it in the toolbar when it looks right.",
            );
        }
    }

    fn handle_text_output(&mut self, output: &canvas::CanvasOutput) {
        if output.text_drag_started.is_some() {
            self.history.checkpoint(&self.document);
            self.pending_text_anchor = None;
            self.text_editor_should_focus = false;
            self.set_status("Moving text annotation...");
        }

        if let Some(drag) = output.text_drag_current {
            if self
                .document
                .set_text_anchor(drag.overlay_index, drag.anchor)
            {
                self.set_status("Moving text annotation...");
            }
        }

        if let Some(drag) = output.text_drag_stopped {
            if self
                .document
                .set_text_anchor(drag.overlay_index, drag.anchor)
            {
                self.set_status("Text moved. Drag existing text again to reposition it.");
            }
            return;
        }

        if let Some(position) = output.clicked {
            self.text_buffer.clear();
            self.pending_text_anchor = Some(position);
            self.text_editor_should_focus = true;
            self.set_status("Text anchor placed. Enter text in the floating editor.");
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
                    &mut self.save_path,
                    &mut self.canvas_state.zoom,
                    self.config.min_zoom,
                    self.config.max_zoom,
                    self.pending_crop.is_some(),
                );

                if let Some(tool) = output.tool_change {
                    self.activate_tool(tool);
                }
                if output.commit_crop {
                    self.commit_crop(ctx);
                }
                if output.cancel_crop {
                    self.pending_crop = None;
                    self.set_status(Self::tool_status_message(ToolKind::Crop));
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
                self.active_tool == ToolKind::Crop,
                self.pending_crop,
                self.active_tool == ToolKind::Text,
                self.pending_text_anchor.is_none(),
            );
            self.handle_canvas_output(output, ctx);
        });

        self.show_text_editor(ctx);
    }
}
