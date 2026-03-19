use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::Duration,
};

use eframe::egui::{
    self, Button, CentralPanel, Context, CornerRadius, Key, KeyboardShortcut, Modifiers, RichText,
    SidePanel, Stroke, TopBottomPanel,
};

use crate::{
    capture::CapturedImage,
    config::AppConfig,
    editor::{CanvasState, Document, HistoryManager, canvas},
    model::{
        overlay::{CropOverlay, OverlayObject, TextAlignment},
        types::{ImagePoint, ImageRect, RgbaColor, StrokeStyle, TextStyle},
    },
    services::{clipboard, ocr, save},
    tools::{self, DraftOverlay, ToolKind},
    ui::{theme, toolbar, topbar},
};

const HIGHLIGHTER_ALPHA: u8 = 112;
const MIN_STROKE_THICKNESS: f32 = 1.0;
const MAX_STROKE_THICKNESS: f32 = 24.0;
const MIN_HIGHLIGHTER_THICKNESS: f32 = 10.0;
const MAX_HIGHLIGHTER_THICKNESS: f32 = 34.0;
const HIGHLIGHTER_DEFAULT_THICKNESS: f32 = 16.0;
const WORKER_POLL_INTERVAL: Duration = Duration::from_millis(50);

struct PendingTextExtraction {
    receiver: Receiver<Result<String, String>>,
    source_label: &'static str,
}

struct PendingSave {
    receiver: Receiver<Result<SaveOutcome, String>>,
}

enum SaveOutcome {
    Saved(PathBuf),
    Cancelled,
}

pub struct SnaptureApp {
    config: AppConfig,
    document: Document,
    history: HistoryManager,
    texture: Option<egui::TextureHandle>,
    texture_revision: u64,
    active_tool: ToolKind,
    draft: Option<DraftOverlay>,
    selected_overlay: Option<usize>,
    pending_crop: Option<ImageRect>,
    pending_text_anchor: Option<ImagePoint>,
    pending_text_alignment: TextAlignment,
    text_editor_should_focus: bool,
    stroke_color: RgbaColor,
    stroke_thickness: f32,
    highlighter_thickness: f32,
    highlighter_alpha: u8,
    text_size: f32,
    text_buffer: String,
    extracted_text: String,
    extracted_text_source: &'static str,
    extracted_text_window_open: bool,
    pending_text_extraction: Option<PendingTextExtraction>,
    pending_save: Option<PendingSave>,
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
        theme::apply(&cc.egui_ctx);

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
            selected_overlay: None,
            pending_crop: None,
            pending_text_anchor: None,
            pending_text_alignment: TextAlignment::Left,
            text_editor_should_focus: false,
            stroke_color: RgbaColor::default(),
            stroke_thickness: 4.0,
            highlighter_thickness: HIGHLIGHTER_DEFAULT_THICKNESS,
            highlighter_alpha: HIGHLIGHTER_ALPHA,
            text_size: 28.0,
            text_buffer: String::new(),
            extracted_text: String::new(),
            extracted_text_source: "screenshot",
            extracted_text_window_open: false,
            pending_text_extraction: None,
            pending_save: None,
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

    fn current_highlighter_style(&self) -> StrokeStyle {
        StrokeStyle::new(
            RgbaColor::from_rgba(
                self.stroke_color.r,
                self.stroke_color.g,
                self.stroke_color.b,
                self.highlighter_alpha,
            ),
            self.highlighter_thickness,
        )
    }

    fn stroke_style_for_tool(&self, tool: ToolKind) -> StrokeStyle {
        match tool {
            ToolKind::Select => self.current_stroke_style(),
            ToolKind::Highlighter => self.current_highlighter_style(),
            ToolKind::Pen
            | ToolKind::Rectangle
            | ToolKind::Arrow
            | ToolKind::Text
            | ToolKind::Crop => self.current_stroke_style(),
        }
    }

    fn current_text_style(&self) -> TextStyle {
        TextStyle::new(self.stroke_color, self.text_size)
    }

    fn set_status(&mut self, message: impl Into<String>) {
        self.status = message.into();
    }

    fn clear_transient_state(&mut self) {
        self.draft = None;
        self.selected_overlay = None;
        self.pending_crop = None;
        self.pending_text_anchor = None;
        self.pending_text_alignment = TextAlignment::Left;
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

        if tool == ToolKind::Highlighter {
            self.highlighter_thickness = self
                .highlighter_thickness
                .clamp(MIN_HIGHLIGHTER_THICKNESS, MAX_HIGHLIGHTER_THICKNESS);
        } else {
            self.stroke_thickness = self
                .stroke_thickness
                .clamp(MIN_STROKE_THICKNESS, MAX_STROKE_THICKNESS);
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
            ToolKind::Select => {
                "Select active. Click an object to select it, drag inside to move it, or drag a handle to resize it."
            }
            ToolKind::Pen => "Pen active. Drag on the image to draw.",
            ToolKind::Highlighter => {
                "Highlighter active. Drag on the image to lay down translucent strokes; increase thickness for broader highlights."
            }
            ToolKind::Rectangle => "Rectangle active. Drag on the image to place a box.",
            ToolKind::Arrow => "Arrow active. Drag on the image to place an arrow.",
            ToolKind::Text => {
                "Text active. Click the image to place a text anchor, type directly on the image, press Enter to apply, or press Shift+Enter for a new line."
            }
            ToolKind::Crop => {
                "Crop active. Resize or move the crop box, then press Enter to commit or Esc to cancel."
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

        overlays
    }

    fn commit_overlay(&mut self, overlay: OverlayObject, status: &'static str) {
        self.history.checkpoint(&self.document);
        self.document.add_overlay(overlay);
        self.set_status(status);
    }

    fn delete_selected_overlay(&mut self) {
        let Some(overlay_index) = self.selected_overlay else {
            return;
        };

        self.history.checkpoint(&self.document);
        if self.document.remove_overlay(overlay_index) {
            self.selected_overlay = None;
            self.set_status("Selected object deleted.");
        }
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

    fn cancel_crop(&mut self) {
        self.activate_tool(ToolKind::Select);
    }

    fn commit_pending_text(&mut self) {
        let Some(anchor) = self.pending_text_anchor else {
            return;
        };

        if let Some(overlay) = tools::text::build_text_overlay(
            anchor,
            self.text_buffer.clone(),
            self.current_text_style(),
            self.pending_text_alignment.clone(),
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

    fn cancel_pending_text(&mut self) {
        self.pending_text_anchor = None;
        self.pending_text_alignment = TextAlignment::Left;
        self.text_editor_should_focus = false;
        self.text_buffer.clear();
        self.set_status(Self::tool_status_message(ToolKind::Text));
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
        if self.pending_save.is_some() {
            self.set_status("A save is already in progress.");
            return;
        }

        let default_path = PathBuf::from(&self.save_path);
        let document = self.document.clone();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = match save::choose_save_path(default_path) {
                Ok(Some(path)) => save::save_document_png(&document, &path)
                    .map(SaveOutcome::Saved)
                    .map_err(|error| error.to_string()),
                Ok(None) => Ok(SaveOutcome::Cancelled),
                Err(error) => Err(error.to_string()),
            };
            let _ = sender.send(result);
        });

        self.pending_save = Some(PendingSave { receiver });
        self.set_status("Choose a save location in the file dialog...");
    }

    fn copy_document(&mut self) {
        match clipboard::copy_document_image(&self.document) {
            Ok(()) => self.set_status("Copied image to clipboard."),
            Err(error) => self.set_status(format!("Clipboard failed: {error}")),
        }
    }

    fn start_text_extraction(&mut self) {
        if self.pending_text_extraction.is_some() {
            self.set_status("Text extraction is already running.");
            return;
        }

        let source_label = if self.pending_crop.is_some() {
            "crop selection"
        } else {
            "screenshot"
        };
        let image = match self.document.base_image_region(self.pending_crop) {
            Ok(image) => image,
            Err(error) => {
                self.set_status(format!("Text extraction failed: {error}"));
                return;
            }
        };

        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = ocr::extract_text(image).map_err(|error| error.to_string());
            let _ = sender.send(result);
        });

        self.pending_text_extraction = Some(PendingTextExtraction {
            receiver,
            source_label,
        });
        self.set_status(format!("Extracting text from the {source_label}..."));
    }

    fn poll_save(&mut self) {
        let Some(pending) = self.pending_save.take() else {
            return;
        };

        match pending.receiver.try_recv() {
            Ok(Ok(SaveOutcome::Saved(path))) => {
                self.save_path = path.display().to_string();
                self.set_status(format!("Saved {}", path.display()));
            }
            Ok(Ok(SaveOutcome::Cancelled)) => {
                self.set_status("Save cancelled.");
            }
            Ok(Err(error)) => {
                self.set_status(format!("Save dialog failed: {error}"));
            }
            Err(TryRecvError::Empty) => {
                self.pending_save = Some(pending);
            }
            Err(TryRecvError::Disconnected) => {
                self.set_status("Save worker stopped unexpectedly.");
            }
        }
    }

    fn poll_text_extraction(&mut self) {
        let Some(pending) = self.pending_text_extraction.take() else {
            return;
        };

        match pending.receiver.try_recv() {
            Ok(Ok(text)) => {
                let non_empty = !text.trim().is_empty();
                let source_label = pending.source_label;

                self.extracted_text = text;
                self.extracted_text_source = source_label;
                self.extracted_text_window_open = true;

                if non_empty {
                    let line_count = self.extracted_text.lines().count();
                    let line_suffix = if line_count == 1 { "" } else { "s" };
                    self.set_status(format!(
                        "Extracted {line_count} line{line_suffix} from the {source_label}."
                    ));
                } else {
                    self.set_status(format!("No text detected in the {source_label}."));
                }
            }
            Ok(Err(error)) => {
                self.set_status(format!(
                    "Text extraction from the {} failed: {error}",
                    pending.source_label
                ));
            }
            Err(TryRecvError::Empty) => {
                self.pending_text_extraction = Some(pending);
            }
            Err(TryRecvError::Disconnected) => {
                self.set_status("Text extraction worker stopped unexpectedly.");
            }
        }
    }

    fn show_extracted_text_window(&mut self, ctx: &Context) {
        if !self.extracted_text_window_open {
            return;
        }

        let mut window_open = self.extracted_text_window_open;
        let mut copy_clicked = false;

        egui::Window::new("Extracted Text")
            .collapsible(false)
            .default_width(460.0)
            .anchor(egui::Align2::RIGHT_TOP, [-16.0, 72.0])
            .frame(theme::floating_frame())
            .open(&mut window_open)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(format!("Source: {}", self.extracted_text_source))
                        .size(11.5)
                        .color(theme::TEXT_MUTED),
                );
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !self.extracted_text.trim().is_empty(),
                            Button::new(RichText::new("Copy Text").strong())
                                .fill(theme::ACCENT)
                                .stroke(Stroke::new(1.0, theme::ACCENT_HOVER))
                                .corner_radius(CornerRadius::same(8)),
                        )
                        .clicked()
                    {
                        copy_clicked = true;
                    }
                });

                if self.extracted_text.trim().is_empty() {
                    ui.label("No text was detected in the selected image area.");
                }

                ui.add(
                    egui::TextEdit::multiline(&mut self.extracted_text)
                        .desired_rows(14)
                        .desired_width(f32::INFINITY)
                        .hint_text("Extracted text will appear here"),
                );
            });

        self.extracted_text_window_open = window_open;

        if copy_clicked {
            ctx.copy_text(self.extracted_text.clone());
            self.set_status("Copied extracted text to clipboard.");
        }
    }

    fn handle_canvas_output(&mut self, output: canvas::CanvasOutput, ctx: &Context) {
        match self.active_tool {
            ToolKind::Select => self.handle_select_output(&output),
            ToolKind::Pen => self.handle_pen_output(&output, ctx),
            ToolKind::Highlighter => self.handle_highlighter_output(&output, ctx),
            ToolKind::Rectangle => self.handle_rectangle_output(&output, ctx),
            ToolKind::Arrow => self.handle_arrow_output(&output, ctx),
            ToolKind::Text => self.handle_text_output(&output, ctx),
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

    fn handle_select_output(&mut self, output: &canvas::CanvasOutput) {
        if output.selection_changed {
            self.selected_overlay = output.selected_overlay;
            if self.selected_overlay.is_some() {
                self.set_status(
                    "Object selected. Drag inside to move it or use the handles to resize it.",
                );
            } else {
                self.set_status(Self::tool_status_message(ToolKind::Select));
            }
        }

        if output.object_transform_started.is_some() {
            self.history.checkpoint(&self.document);
            self.pending_text_anchor = None;
            self.text_editor_should_focus = false;
            self.set_status("Transforming selection...");
        }

        if let Some(transform) = &output.object_transform_current {
            if self
                .document
                .set_overlay(transform.overlay_index, transform.overlay.clone())
            {
                self.selected_overlay = Some(transform.overlay_index);
            }
        }

        if let Some(transform) = &output.object_transform_stopped {
            if self
                .document
                .set_overlay(transform.overlay_index, transform.overlay.clone())
            {
                self.selected_overlay = Some(transform.overlay_index);
                self.set_status(
                    "Object updated. Drag inside to move it again or use the handles to resize it.",
                );
            }
        }
    }

    fn handle_highlighter_output(&mut self, output: &canvas::CanvasOutput, ctx: &Context) {
        self.handle_shape_output(
            output,
            ctx,
            ToolKind::Highlighter,
            "Highlight added. Drag again to mark another area.",
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
            self.draft = tools::begin_drag(tool, start, self.stroke_style_for_tool(tool));
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
            self.set_status("Crop box updated. Press Enter to commit or Esc to cancel.");
        }
    }

    fn handle_text_output(&mut self, output: &canvas::CanvasOutput, ctx: &Context) {
        if output.text_submit_requested {
            self.commit_pending_text();
            ctx.request_repaint();
            return;
        }

        if output.text_cancel_requested {
            self.cancel_pending_text();
            ctx.request_repaint();
            return;
        }

        if output.text_drag_started.is_some() {
            self.history.checkpoint(&self.document);
            self.pending_text_anchor = None;
            self.pending_text_alignment = TextAlignment::Left;
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
            self.pending_text_alignment = TextAlignment::Left;
            self.text_editor_should_focus = true;
            self.set_status(
                "Text anchor placed. Type directly on the image. Press Enter to apply or Shift+Enter for a new line.",
            );
            ctx.request_repaint();
        }
    }

    fn handle_shortcuts(&mut self, ctx: &Context) {
        let undo_shortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::Z);
        let redo_shortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::Y);
        let redo_alt_shortcut = KeyboardShortcut::new(Modifiers::CTRL | Modifiers::SHIFT, Key::Z);
        let save_shortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::S);
        let copy_shortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::C);
        let global_copy_shortcut_enabled = !ctx.wants_keyboard_input();
        let crop_shortcuts_enabled = self.active_tool == ToolKind::Crop
            && self.pending_crop.is_some()
            && global_copy_shortcut_enabled;
        let text_shortcuts_enabled =
            self.active_tool == ToolKind::Text && self.pending_text_anchor.is_some();
        let mut commit_text_requested = false;
        let mut cancel_text_requested = false;

        if text_shortcuts_enabled {
            ctx.input_mut(|input| {
                input.events.retain(|event| match event {
                    egui::Event::Key {
                        key: Key::Enter,
                        pressed: true,
                        repeat: false,
                        modifiers,
                        ..
                    } if !modifiers.alt
                        && !modifiers.ctrl
                        && !modifiers.shift
                        && !modifiers.command
                        && !modifiers.mac_cmd =>
                    {
                        commit_text_requested = true;
                        false
                    }
                    egui::Event::Key {
                        key: Key::Escape,
                        pressed: true,
                        repeat: false,
                        ..
                    } => {
                        cancel_text_requested = true;
                        false
                    }
                    _ => true,
                });
            });
        }

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
        if global_copy_shortcut_enabled
            && ctx.input_mut(|input| {
                let mut consumed_copy_event = false;
                input.events.retain(|event| {
                    let is_copy_event = matches!(event, egui::Event::Copy);
                    consumed_copy_event |= is_copy_event;
                    !is_copy_event
                });

                input.consume_shortcut(&copy_shortcut) || consumed_copy_event
            })
        {
            self.copy_document();
        }
        if crop_shortcuts_enabled
            && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter))
        {
            self.commit_crop(ctx);
        }
        if crop_shortcuts_enabled
            && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.cancel_crop();
        }
        if text_shortcuts_enabled && commit_text_requested {
            self.commit_pending_text();
            ctx.request_repaint();
        }
        if text_shortcuts_enabled && cancel_text_requested {
            self.cancel_pending_text();
            ctx.request_repaint();
        }
    }
}

impl eframe::App for SnaptureApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.poll_save();
        self.poll_text_extraction();
        self.handle_shortcuts(ctx);

        if self.pending_save.is_some() || self.pending_text_extraction.is_some() {
            ctx.request_repaint_after(WORKER_POLL_INTERVAL);
        }

        TopBottomPanel::top("topbar")
            .frame(theme::topbar_frame())
            .show(ctx, |ui| {
                let output = topbar::show(
                    ui,
                    self.history.can_undo(),
                    self.history.can_redo(),
                    self.pending_text_extraction.is_some(),
                    self.pending_save.is_some(),
                    &self.status,
                );

                if output.save_clicked {
                    self.save_document();
                }
                if output.copy_clicked {
                    self.copy_document();
                }
                if output.extract_text_clicked {
                    self.start_text_extraction();
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
            .frame(theme::sidebar_frame())
            .resizable(false)
            .default_width(260.0)
            .show(ctx, |ui| {
                let output = toolbar::show(
                    ui,
                    self.active_tool,
                    &mut self.stroke_color,
                    &mut self.stroke_thickness,
                    &mut self.highlighter_thickness,
                    &mut self.highlighter_alpha,
                    &mut self.text_size,
                    &mut self.save_path,
                    &mut self.canvas_state.zoom,
                    self.config.min_zoom,
                    self.config.max_zoom,
                    self.selected_overlay.is_some(),
                    self.pending_crop.is_some(),
                );

                if let Some(tool) = output.tool_change {
                    self.activate_tool(tool);
                }
                if output.delete_selected {
                    self.delete_selected_overlay();
                }
                if output.commit_crop {
                    self.commit_crop(ctx);
                }
                if output.cancel_crop {
                    self.cancel_crop();
                }
            });

        CentralPanel::default()
            .frame(theme::central_frame())
            .show(ctx, |ui| {
                let preview_overlays = self.preview_overlays();
                let pending_text_style = self.current_text_style();
                let output = canvas::show(
                    ui,
                    &self.document,
                    self.texture.as_ref(),
                    &mut self.canvas_state,
                    &preview_overlays,
                    self.active_tool == ToolKind::Crop,
                    self.pending_crop,
                    self.active_tool == ToolKind::Text,
                    self.pending_text_anchor,
                    self.pending_text_alignment.clone(),
                    pending_text_style,
                    &mut self.text_buffer,
                    &mut self.text_editor_should_focus,
                    self.pending_text_anchor.is_none(),
                    self.active_tool == ToolKind::Select,
                    self.selected_overlay,
                );
                self.handle_canvas_output(output, ctx);
            });

        self.show_extracted_text_window(ctx);
    }
}
