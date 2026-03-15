use eframe::egui::{
    self, Align2, Color32, CursorIcon, FontId, Pos2, Rect, Sense, Shape, Stroke,
    TextureHandle, pos2, vec2,
};

use crate::{
    editor::document::Document,
    model::{
        overlay::{CropOverlay, OverlayObject},
        types::{ImagePoint, ImageRect},
    },
};

#[derive(Clone, Debug)]
pub struct CanvasState {
    pub zoom: f32,
    crop_interaction: Option<CropInteractionState>,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            crop_interaction: None,
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

pub fn show(
    ui: &mut egui::Ui,
    document: &Document,
    texture: Option<&TextureHandle>,
    state: &mut CanvasState,
    preview_overlays: &[OverlayObject],
    crop_tool_active: bool,
    pending_crop: Option<ImageRect>,
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

    let hover_crop_interaction = if crop_tool_active {
        pending_crop.and_then(|rect| {
            pointer_pos.and_then(|pos| crop_interaction_at(transform.image_rect_to_screen(rect), pos))
        })
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

        if let Some(interaction) = crop_drag_started {
            state.crop_interaction = Some(interaction);
        } else if !crop_tool_active {
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
    } else if !crop_tool_active {
        if response.dragged() {
            output.drag_current = pointer_pos.map(|pos| transform.screen_to_image_clamped(pos));
        }

        if response.drag_stopped() {
            output.drag_stopped = pointer_pos.map(|pos| transform.screen_to_image_clamped(pos));
        }
    }

    if response.clicked() {
        output.clicked = pointer_pos.and_then(|pos| transform.screen_to_image(pos));
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
            let min_x = (interaction.initial_rect.min.x + dx).clamp(0.0, (width_limit - width).max(0.0));
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
