use eframe::egui::{
    self, Align2, Color32, CursorIcon, FontId, Pos2, Rect, Sense, Shape, Stroke, TextureHandle,
    pos2, vec2,
};

use crate::{
    editor::document::Document,
    model::{
        overlay::{CropOverlay, OverlayObject, TextOverlay},
        types::{ImagePoint, ImageRect},
    },
};

#[derive(Clone, Debug)]
pub struct CanvasState {
    pub zoom: f32,
    crop_interaction: Option<CropInteractionState>,
    text_interaction: Option<TextInteractionState>,
    selection_interaction: Option<SelectionInteractionState>,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            crop_interaction: None,
            text_interaction: None,
            selection_interaction: None,
        }
    }
}

#[derive(Default)]
pub struct CanvasOutput {
    pub hover_position: Option<ImagePoint>,
    pub drag_started: Option<ImagePoint>,
    pub drag_current: Option<ImagePoint>,
    pub drag_stopped: Option<ImagePoint>,
    pub clicked: Option<ImagePoint>,
    pub crop_rect: Option<ImageRect>,
    pub text_drag_started: Option<usize>,
    pub text_drag_current: Option<TextDragOutput>,
    pub text_drag_stopped: Option<TextDragOutput>,
    pub selection_changed: bool,
    pub selected_overlay: Option<usize>,
    pub object_transform_started: Option<usize>,
    pub object_transform_current: Option<OverlayTransformOutput>,
    pub object_transform_stopped: Option<OverlayTransformOutput>,
}

#[derive(Clone, Copy, Debug)]
pub struct TextDragOutput {
    pub overlay_index: usize,
    pub anchor: ImagePoint,
}

#[derive(Clone, Debug)]
pub struct OverlayTransformOutput {
    pub overlay_index: usize,
    pub overlay: OverlayObject,
}

#[derive(Clone, Copy)]
struct ScreenTransform {
    image_rect: Rect,
    scale: f32,
    image_size: [u32; 2],
}

impl ScreenTransform {
    fn image_to_screen(self, point: ImagePoint) -> Pos2 {
        pos2(
            self.image_rect.min.x + point.x * self.scale,
            self.image_rect.min.y + point.y * self.scale,
        )
    }

    fn image_rect_to_screen(self, rect: ImageRect) -> Rect {
        Rect::from_min_max(
            self.image_to_screen(rect.min),
            self.image_to_screen(rect.max),
        )
    }

    fn screen_to_image(self, point: Pos2) -> Option<ImagePoint> {
        if !self.image_rect.contains(point) {
            return None;
        }

        Some(self.screen_to_image_clamped(point))
    }

    fn screen_to_image_clamped(self, point: Pos2) -> ImagePoint {
        let x =
            ((point.x - self.image_rect.min.x) / self.scale).clamp(0.0, self.image_size[0] as f32);
        let y =
            ((point.y - self.image_rect.min.y) / self.scale).clamp(0.0, self.image_size[1] as f32);
        ImagePoint::new(x, y)
    }
}

#[derive(Clone, Copy, Debug)]
struct CropInteractionState {
    kind: CropInteractionKind,
    origin: ImagePoint,
    initial_rect: ImageRect,
}

#[derive(Clone, Copy, Debug)]
enum CropInteractionKind {
    Move,
    Resize(CropHandle),
}

#[derive(Clone, Copy, Debug)]
enum CropHandle {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

#[derive(Clone, Copy, Debug)]
struct TextInteractionState {
    overlay_index: usize,
    origin: ImagePoint,
    initial_anchor: ImagePoint,
    bounds_offset: ImageRect,
}

#[derive(Clone, Debug)]
struct SelectionInteractionState {
    overlay_index: usize,
    interaction: CropInteractionState,
    initial_overlay: OverlayObject,
}

pub fn show(
    ui: &mut egui::Ui,
    document: &Document,
    texture: Option<&TextureHandle>,
    state: &mut CanvasState,
    preview_overlays: &[OverlayObject],
    crop_tool_active: bool,
    pending_crop: Option<ImageRect>,
    text_tool_active: bool,
    text_drag_enabled: bool,
    selection_tool_active: bool,
    selected_overlay: Option<usize>,
) -> CanvasOutput {
    let available = ui.available_size_before_wrap();
    let (response, painter) = ui.allocate_painter(available, Sense::click_and_drag());
    let mut output = CanvasOutput::default();

    painter.rect_filled(response.rect, 0.0, Color32::from_gray(26));

    let image_size = document.image_size();
    let image_size_vec = vec2(image_size[0] as f32, image_size[1] as f32);
    let fit_scale = (response.rect.width() / image_size_vec.x)
        .min(response.rect.height() / image_size_vec.y)
        .min(1.0);
    let scale = fit_scale * state.zoom.max(0.01);
    let scaled_size = image_size_vec * scale;
    let image_rect = Rect::from_center_size(response.rect.center(), scaled_size);
    let transform = ScreenTransform {
        image_rect,
        scale,
        image_size,
    };

    painter.rect_filled(image_rect.expand(6.0), 4.0, Color32::from_gray(18));

    if let Some(texture) = texture {
        painter.image(
            texture.id(),
            image_rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        painter.text(
            response.rect.center(),
            Align2::CENTER_CENTER,
            "No image loaded",
            FontId::proportional(18.0),
            Color32::LIGHT_GRAY,
        );
    }

    for overlay in document.overlays() {
        paint_overlay(&painter, transform, overlay, false);
    }

    for overlay in preview_overlays {
        paint_overlay(&painter, transform, overlay, true);
    }

    let pointer_pos = response
        .interact_pointer_pos()
        .or_else(|| ui.ctx().pointer_latest_pos());
    let press_origin = ui.input(|input| input.pointer.press_origin());

    output.hover_position = pointer_pos.and_then(|pos| transform.screen_to_image(pos));

    if !crop_tool_active {
        state.crop_interaction = None;
    }
    if !text_tool_active || !text_drag_enabled {
        state.text_interaction = None;
    }
    if !selection_tool_active {
        state.selection_interaction = None;
    }

    let selected_overlay_bounds = if selection_tool_active {
        selected_overlay.and_then(|overlay_index| {
            overlay_bounds_for_index(&painter, transform, document, overlay_index)
        })
    } else {
        None
    };

    let hover_crop_interaction = if crop_tool_active {
        pending_crop.and_then(|rect| {
            pointer_pos
                .and_then(|pos| crop_interaction_at(transform.image_rect_to_screen(rect), pos))
        })
    } else {
        None
    };
    let hover_text_interaction = if text_tool_active && text_drag_enabled {
        output
            .hover_position
            .and_then(|point| text_overlay_at(&painter, transform, document, point))
    } else {
        None
    };
    let hover_selection_interaction = if selection_tool_active {
        selected_overlay_bounds.and_then(|bounds| {
            pointer_pos
                .and_then(|pos| crop_interaction_at(transform.image_rect_to_screen(bounds), pos))
        })
    } else {
        None
    };
    let hover_overlay = if selection_tool_active {
        output
            .hover_position
            .and_then(|point| overlay_at(&painter, transform, document, point))
    } else {
        None
    };

    if crop_tool_active {
        let cursor = state
            .crop_interaction
            .map(|interaction| crop_cursor(interaction.kind))
            .or_else(|| hover_crop_interaction.map(crop_cursor));
        if let Some(cursor) = cursor {
            ui.output_mut(|output| output.cursor_icon = cursor);
        }
    } else if selection_tool_active {
        let cursor = state
            .selection_interaction
            .as_ref()
            .map(|interaction| crop_cursor(interaction.interaction.kind))
            .or_else(|| hover_selection_interaction.map(crop_cursor))
            .or_else(|| hover_overlay.map(|_| CursorIcon::Grab));
        if let Some(cursor) = cursor {
            ui.output_mut(|output| output.cursor_icon = cursor);
        }
    } else if text_tool_active && text_drag_enabled {
        let cursor = if state.text_interaction.is_some() {
            Some(CursorIcon::Grabbing)
        } else if hover_text_interaction.is_some() {
            Some(CursorIcon::Grab)
        } else {
            None
        };
        if let Some(cursor) = cursor {
            ui.output_mut(|output| output.cursor_icon = cursor);
        }
    }

    if response.drag_started() {
        let crop_drag_started = if crop_tool_active {
            if let (Some(rect), Some(origin_screen), Some(origin_image)) = (
                pending_crop,
                press_origin,
                press_origin.map(|pos| transform.screen_to_image_clamped(pos)),
            ) {
                crop_interaction_at(transform.image_rect_to_screen(rect), origin_screen).map(
                    |kind| CropInteractionState {
                        kind,
                        origin: origin_image,
                        initial_rect: rect,
                    },
                )
            } else {
                None
            }
        } else {
            None
        };
        let selection_drag_started = if selection_tool_active {
            if let (Some(origin_screen), Some(origin_image), Some(bounds), Some(index)) = (
                press_origin,
                press_origin.map(|pos| transform.screen_to_image_clamped(pos)),
                selected_overlay_bounds,
                selected_overlay,
            ) {
                crop_interaction_at(transform.image_rect_to_screen(bounds), origin_screen).and_then(
                    |kind| {
                        document
                            .overlays()
                            .get(index)
                            .cloned()
                            .map(|initial_overlay| SelectionInteractionState {
                                overlay_index: index,
                                interaction: CropInteractionState {
                                    kind,
                                    origin: origin_image,
                                    initial_rect: bounds,
                                },
                                initial_overlay,
                            })
                    },
                )
            } else {
                None
            }
        } else {
            None
        };
        let selection_overlay_drag_started =
            if selection_tool_active && selection_drag_started.is_none() {
                press_origin
                    .and_then(|pos| transform.screen_to_image(pos))
                    .and_then(|point| {
                        overlay_at(&painter, transform, document, point)
                            .map(|(overlay_index, bounds)| (point, overlay_index, bounds))
                    })
                    .and_then(|(point, overlay_index, bounds)| {
                        document
                            .overlays()
                            .get(overlay_index)
                            .cloned()
                            .map(|initial_overlay| SelectionInteractionState {
                                overlay_index,
                                interaction: CropInteractionState {
                                    kind: CropInteractionKind::Move,
                                    origin: point,
                                    initial_rect: bounds,
                                },
                                initial_overlay,
                            })
                    })
            } else {
                None
            };
        let text_drag_started = if text_tool_active && text_drag_enabled {
            press_origin
                .and_then(|pos| transform.screen_to_image(pos))
                .and_then(|point| {
                    text_overlay_at(&painter, transform, document, point).map(
                        |(overlay_index, initial_anchor, bounds_offset)| TextInteractionState {
                            overlay_index,
                            origin: point,
                            initial_anchor,
                            bounds_offset,
                        },
                    )
                })
        } else {
            None
        };

        if let Some(interaction) = crop_drag_started {
            state.crop_interaction = Some(interaction);
        } else if let Some(interaction) = selection_drag_started {
            output.object_transform_started = Some(interaction.overlay_index);
            state.selection_interaction = Some(interaction);
        } else if let Some(interaction) = selection_overlay_drag_started {
            output.selection_changed = true;
            output.selected_overlay = Some(interaction.overlay_index);
            output.object_transform_started = Some(interaction.overlay_index);
            state.selection_interaction = Some(interaction);
        } else if let Some(interaction) = text_drag_started {
            output.text_drag_started = Some(interaction.overlay_index);
            state.text_interaction = Some(interaction);
        } else if !crop_tool_active && !text_tool_active && !selection_tool_active {
            output.drag_started = press_origin.and_then(|pos| transform.screen_to_image(pos));
        }
    }

    if let Some(interaction) = state.crop_interaction {
        if response.dragged() || response.drag_stopped() {
            if let Some(current) = pointer_pos.map(|pos| transform.screen_to_image_clamped(pos)) {
                output.crop_rect = Some(apply_crop_interaction(interaction, current, image_size));
            }
        }

        if response.drag_stopped() {
            state.crop_interaction = None;
        }
    } else if let Some(interaction) = &state.selection_interaction {
        if response.dragged() || response.drag_stopped() {
            if let Some(current) = pointer_pos.map(|pos| transform.screen_to_image_clamped(pos)) {
                let target_bounds =
                    apply_crop_interaction(interaction.interaction, current, image_size);
                let overlay = interaction
                    .initial_overlay
                    .transformed_to_bounds(interaction.interaction.initial_rect, target_bounds);
                let transformed = OverlayTransformOutput {
                    overlay_index: interaction.overlay_index,
                    overlay,
                };
                if response.drag_stopped() {
                    output.object_transform_stopped = Some(transformed);
                } else {
                    output.object_transform_current = Some(transformed);
                }
            }
        }

        if response.drag_stopped() {
            state.selection_interaction = None;
        }
    } else if let Some(interaction) = state.text_interaction {
        if response.dragged() || response.drag_stopped() {
            if let Some(current) = pointer_pos.map(|pos| transform.screen_to_image_clamped(pos)) {
                let anchor = apply_text_interaction(interaction, current, image_size);
                let drag = TextDragOutput {
                    overlay_index: interaction.overlay_index,
                    anchor,
                };
                if response.drag_stopped() {
                    output.text_drag_stopped = Some(drag);
                } else {
                    output.text_drag_current = Some(drag);
                }
            }
        }

        if response.drag_stopped() {
            state.text_interaction = None;
        }
    } else if !crop_tool_active && !text_tool_active && !selection_tool_active {
        if response.dragged() {
            output.drag_current = pointer_pos.map(|pos| transform.screen_to_image_clamped(pos));
        }

        if response.drag_stopped() {
            output.drag_stopped = pointer_pos.map(|pos| transform.screen_to_image_clamped(pos));
        }
    }

    let clicked_existing_text = text_tool_active
        && text_drag_enabled
        && output
            .hover_position
            .and_then(|point| text_overlay_at(&painter, transform, document, point))
            .is_some();
    let clicked_overlay = if selection_tool_active {
        output
            .hover_position
            .and_then(|point| overlay_at(&painter, transform, document, point))
            .map(|(index, _)| index)
    } else {
        None
    };

    if response.clicked() && selection_tool_active {
        output.selection_changed = true;
        output.selected_overlay = clicked_overlay;
    } else if response.clicked() && !clicked_existing_text {
        output.clicked = pointer_pos.and_then(|pos| transform.screen_to_image(pos));
    }

    if selection_tool_active {
        if let Some(bounds) = selected_overlay_bounds {
            paint_selection_overlay(&painter, transform, bounds);
        }
    }

    output
}

fn paint_overlay(
    painter: &egui::Painter,
    transform: ScreenTransform,
    overlay: &OverlayObject,
    preview: bool,
) {
    match overlay {
        OverlayObject::Pen(stroke) => {
            let points: Vec<Pos2> = stroke
                .points
                .iter()
                .map(|point| transform.image_to_screen(*point))
                .collect();
            if !points.is_empty() {
                painter.add(Shape::line(
                    points,
                    Stroke::new(
                        (stroke.style.thickness * transform.scale).max(1.0),
                        stroke.style.color.to_egui(),
                    ),
                ));
            }
        }
        OverlayObject::Rectangle(rectangle) => {
            painter.rect_stroke(
                transform.image_rect_to_screen(rectangle.rect),
                0.0,
                Stroke::new(
                    (rectangle.style.thickness * transform.scale).max(1.0),
                    rectangle.style.color.to_egui(),
                ),
                egui::StrokeKind::Inside,
            );
        }
        OverlayObject::Arrow(arrow) => {
            let start = transform.image_to_screen(arrow.start);
            let end = transform.image_to_screen(arrow.end);
            let stroke = Stroke::new(
                (arrow.style.thickness * transform.scale).max(1.0),
                arrow.style.color.to_egui(),
            );

            painter.line_segment([start, end], stroke);

            let direction = end - start;
            let length = direction.length().max(1.0);
            let unit = direction / length;
            let head_len = (stroke.width * 4.0).max(14.0);
            let head_width = head_len * 0.35;
            let base = end - unit * head_len;
            let perp = vec2(-unit.y, unit.x);
            let left = base + perp * head_width;
            let right = base - perp * head_width;

            painter.line_segment([end, left], stroke);
            painter.line_segment([end, right], stroke);
            painter.add(Shape::convex_polygon(
                vec![end, left, right],
                if preview {
                    arrow.style.color.to_egui().gamma_multiply(0.18)
                } else {
                    Color32::TRANSPARENT
                },
                Stroke::NONE,
            ));
        }
        OverlayObject::Text(text) => {
            painter.text(
                transform.image_to_screen(text.anchor),
                Align2::LEFT_TOP,
                &text.text,
                FontId::proportional((text.style.size * transform.scale).max(10.0)),
                text.style.color.to_egui(),
            );
        }
        OverlayObject::Crop(crop) => paint_crop_overlay(painter, transform, crop, preview),
    }
}

fn paint_selection_overlay(painter: &egui::Painter, transform: ScreenTransform, bounds: ImageRect) {
    let screen_rect = transform.image_rect_to_screen(bounds);
    let border_color = Color32::from_rgb(84, 160, 255);

    painter.rect_stroke(
        screen_rect,
        0.0,
        Stroke::new(2.0, border_color),
        egui::StrokeKind::Inside,
    );

    for center in crop_handle_positions(screen_rect) {
        let handle_rect = Rect::from_center_size(center, vec2(9.0, 9.0));
        painter.rect_filled(handle_rect, 2.0, Color32::WHITE);
        painter.rect_stroke(
            handle_rect,
            2.0,
            Stroke::new(1.5, border_color),
            egui::StrokeKind::Inside,
        );
    }
}

fn paint_crop_overlay(
    painter: &egui::Painter,
    transform: ScreenTransform,
    crop: &CropOverlay,
    preview: bool,
) {
    let screen_rect = transform.image_rect_to_screen(crop.rect);
    let shade = Color32::from_rgba_unmultiplied(0, 0, 0, 96);
    let border_color = if preview {
        Color32::from_rgb(80, 220, 140)
    } else {
        Color32::from_rgb(64, 196, 120)
    };

    let top = Rect::from_min_max(
        transform.image_rect.min,
        pos2(transform.image_rect.max.x, screen_rect.min.y),
    );
    let bottom = Rect::from_min_max(
        pos2(transform.image_rect.min.x, screen_rect.max.y),
        transform.image_rect.max,
    );
    let left = Rect::from_min_max(
        pos2(transform.image_rect.min.x, screen_rect.min.y),
        pos2(screen_rect.min.x, screen_rect.max.y),
    );
    let right = Rect::from_min_max(
        pos2(screen_rect.max.x, screen_rect.min.y),
        pos2(transform.image_rect.max.x, screen_rect.max.y),
    );

    for rect in [top, bottom, left, right] {
        if rect.is_positive() {
            painter.rect_filled(rect, 0.0, shade);
        }
    }

    painter.rect_stroke(
        screen_rect,
        0.0,
        Stroke::new(if preview { 3.0 } else { 2.0 }, border_color),
        egui::StrokeKind::Inside,
    );

    if preview {
        for center in crop_handle_positions(screen_rect) {
            let handle_rect = Rect::from_center_size(center, vec2(9.0, 9.0));
            painter.rect_filled(handle_rect, 2.0, Color32::WHITE);
            painter.rect_stroke(
                handle_rect,
                2.0,
                Stroke::new(1.5, border_color),
                egui::StrokeKind::Inside,
            );
        }
    }
}

fn crop_interaction_at(screen_rect: Rect, point: Pos2) -> Option<CropInteractionKind> {
    let margin = 10.0;
    let near_left = (point.x - screen_rect.min.x).abs() <= margin;
    let near_right = (point.x - screen_rect.max.x).abs() <= margin;
    let near_top = (point.y - screen_rect.min.y).abs() <= margin;
    let near_bottom = (point.y - screen_rect.max.y).abs() <= margin;
    let within_x = point.x >= screen_rect.min.x - margin && point.x <= screen_rect.max.x + margin;
    let within_y = point.y >= screen_rect.min.y - margin && point.y <= screen_rect.max.y + margin;

    if near_left && near_top {
        return Some(CropInteractionKind::Resize(CropHandle::NorthWest));
    }
    if near_right && near_top {
        return Some(CropInteractionKind::Resize(CropHandle::NorthEast));
    }
    if near_left && near_bottom {
        return Some(CropInteractionKind::Resize(CropHandle::SouthWest));
    }
    if near_right && near_bottom {
        return Some(CropInteractionKind::Resize(CropHandle::SouthEast));
    }
    if near_top && within_x {
        return Some(CropInteractionKind::Resize(CropHandle::North));
    }
    if near_bottom && within_x {
        return Some(CropInteractionKind::Resize(CropHandle::South));
    }
    if near_left && within_y {
        return Some(CropInteractionKind::Resize(CropHandle::West));
    }
    if near_right && within_y {
        return Some(CropInteractionKind::Resize(CropHandle::East));
    }
    if screen_rect.contains(point) {
        return Some(CropInteractionKind::Move);
    }

    None
}

fn text_overlay_at(
    painter: &egui::Painter,
    transform: ScreenTransform,
    document: &Document,
    point: ImagePoint,
) -> Option<(usize, ImagePoint, ImageRect)> {
    document
        .overlays()
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, overlay)| match overlay {
            OverlayObject::Text(text) => {
                let bounds_offset = text_bounds_offset(painter, transform, text);
                let bounds = bounds_offset.translated(text.anchor.x, text.anchor.y);
                bounds
                    .contains(point)
                    .then_some((index, text.anchor, bounds_offset))
            }
            _ => None,
        })
}

fn overlay_at(
    painter: &egui::Painter,
    transform: ScreenTransform,
    document: &Document,
    point: ImagePoint,
) -> Option<(usize, ImageRect)> {
    document
        .overlays()
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, _)| {
            overlay_bounds_for_index(painter, transform, document, index)
                .filter(|bounds| bounds.contains(point))
                .map(|bounds| (index, bounds))
        })
}

fn overlay_bounds_for_index(
    painter: &egui::Painter,
    transform: ScreenTransform,
    document: &Document,
    overlay_index: usize,
) -> Option<ImageRect> {
    let overlay = document.overlays().get(overlay_index)?;

    Some(match overlay {
        OverlayObject::Text(text) => {
            text_bounds_offset(painter, transform, text).translated(text.anchor.x, text.anchor.y)
        }
        _ => overlay.bounds(),
    })
}

fn apply_text_interaction(
    interaction: TextInteractionState,
    current: ImagePoint,
    image_size: [u32; 2],
) -> ImagePoint {
    let dx = current.x - interaction.origin.x;
    let dy = current.y - interaction.origin.y;
    let min_x = -interaction.bounds_offset.min.x;
    let max_x = image_size[0] as f32 - interaction.bounds_offset.max.x;
    let min_y = -interaction.bounds_offset.min.y;
    let max_y = image_size[1] as f32 - interaction.bounds_offset.max.y;

    ImagePoint::new(
        clamp_with_unordered_bounds(interaction.initial_anchor.x + dx, min_x, max_x),
        clamp_with_unordered_bounds(interaction.initial_anchor.y + dy, min_y, max_y),
    )
}

fn text_bounds_offset(
    painter: &egui::Painter,
    transform: ScreenTransform,
    text: &TextOverlay,
) -> ImageRect {
    let galley = painter.layout_no_wrap(
        text.text.clone(),
        FontId::proportional((text.style.size * transform.scale).max(10.0)),
        text.style.color.to_egui(),
    );
    let bounds = if galley.mesh_bounds.is_positive() {
        galley.mesh_bounds
    } else {
        galley.rect
    };

    ImageRect::from_points(
        ImagePoint::new(
            bounds.min.x / transform.scale,
            bounds.min.y / transform.scale,
        ),
        ImagePoint::new(
            bounds.max.x / transform.scale,
            bounds.max.y / transform.scale,
        ),
    )
}

fn clamp_with_unordered_bounds(value: f32, a: f32, b: f32) -> f32 {
    if a <= b {
        value.clamp(a, b)
    } else {
        value.clamp(b, a)
    }
}

fn crop_cursor(interaction: CropInteractionKind) -> CursorIcon {
    match interaction {
        CropInteractionKind::Move => CursorIcon::Grab,
        CropInteractionKind::Resize(CropHandle::North | CropHandle::South) => {
            CursorIcon::ResizeVertical
        }
        CropInteractionKind::Resize(CropHandle::East | CropHandle::West) => {
            CursorIcon::ResizeHorizontal
        }
        CropInteractionKind::Resize(CropHandle::NorthWest | CropHandle::SouthEast) => {
            CursorIcon::ResizeNwSe
        }
        CropInteractionKind::Resize(CropHandle::NorthEast | CropHandle::SouthWest) => {
            CursorIcon::ResizeNeSw
        }
    }
}

fn apply_crop_interaction(
    interaction: CropInteractionState,
    current: ImagePoint,
    image_size: [u32; 2],
) -> ImageRect {
    let width_limit = image_size[0] as f32;
    let height_limit = image_size[1] as f32;
    let min_size = 1.0;

    match interaction.kind {
        CropInteractionKind::Move => {
            let width = interaction.initial_rect.width();
            let height = interaction.initial_rect.height();
            let dx = current.x - interaction.origin.x;
            let dy = current.y - interaction.origin.y;
            let min_x =
                (interaction.initial_rect.min.x + dx).clamp(0.0, (width_limit - width).max(0.0));
            let min_y =
                (interaction.initial_rect.min.y + dy).clamp(0.0, (height_limit - height).max(0.0));

            ImageRect::from_points(
                ImagePoint::new(min_x, min_y),
                ImagePoint::new(min_x + width, min_y + height),
            )
        }
        CropInteractionKind::Resize(handle) => {
            let rect = interaction.initial_rect.normalized();
            let mut min = rect.min;
            let mut max = rect.max;

            match handle {
                CropHandle::North => {
                    min.y = current.y.clamp(0.0, rect.max.y - min_size);
                }
                CropHandle::South => {
                    max.y = current.y.clamp(rect.min.y + min_size, height_limit);
                }
                CropHandle::East => {
                    max.x = current.x.clamp(rect.min.x + min_size, width_limit);
                }
                CropHandle::West => {
                    min.x = current.x.clamp(0.0, rect.max.x - min_size);
                }
                CropHandle::NorthEast => {
                    min.y = current.y.clamp(0.0, rect.max.y - min_size);
                    max.x = current.x.clamp(rect.min.x + min_size, width_limit);
                }
                CropHandle::NorthWest => {
                    min.y = current.y.clamp(0.0, rect.max.y - min_size);
                    min.x = current.x.clamp(0.0, rect.max.x - min_size);
                }
                CropHandle::SouthEast => {
                    max.y = current.y.clamp(rect.min.y + min_size, height_limit);
                    max.x = current.x.clamp(rect.min.x + min_size, width_limit);
                }
                CropHandle::SouthWest => {
                    max.y = current.y.clamp(rect.min.y + min_size, height_limit);
                    min.x = current.x.clamp(0.0, rect.max.x - min_size);
                }
            }

            ImageRect { min, max }.normalized()
        }
    }
}

fn crop_handle_positions(screen_rect: Rect) -> [Pos2; 8] {
    let center_x = (screen_rect.min.x + screen_rect.max.x) * 0.5;
    let center_y = (screen_rect.min.y + screen_rect.max.y) * 0.5;

    [
        screen_rect.min,
        pos2(center_x, screen_rect.min.y),
        pos2(screen_rect.max.x, screen_rect.min.y),
        pos2(screen_rect.max.x, center_y),
        screen_rect.max,
        pos2(center_x, screen_rect.max.y),
        pos2(screen_rect.min.x, screen_rect.max.y),
        pos2(screen_rect.min.x, center_y),
    ]
}
