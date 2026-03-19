use std::{f32::consts::PI, fs, path::Path};

use ab_glyph::FontArc;
use fontdb::{Database, Family, Query, Source};
use image::{Pixel, RgbaImage, imageops};
use imageproc::{
    drawing::{
        draw_antialiased_line_segment_mut, draw_antialiased_polygon_mut, draw_text_mut, text_size,
    },
    pixelops::interpolate,
    point::Point,
};

use crate::{
    error::{AppResult, SnaptureError},
    model::{
        overlay::{ArrowOverlay, OverlayObject, PenStrokeOverlay, RectangleOverlay, TextOverlay},
        types::{ImagePoint, ImageRect},
    },
};

const EXPORT_SUPERSAMPLE_3X_MAX_PIXELS: u64 = 3_000_000;
const EXPORT_SUPERSAMPLE_2X_MAX_PIXELS: u64 = 8_500_000;

#[derive(Clone)]
pub struct Document {
    base_image: RgbaImage,
    overlays: Vec<OverlayObject>,
    source_uri: Option<String>,
}

impl Document {
    pub fn from_image(image: RgbaImage, source_uri: Option<String>) -> Self {
        Self {
            base_image: image,
            overlays: Vec::new(),
            source_uri,
        }
    }

    pub fn base_image(&self) -> &RgbaImage {
        &self.base_image
    }

    pub fn overlays(&self) -> &[OverlayObject] {
        &self.overlays
    }

    pub fn source_uri(&self) -> Option<&String> {
        self.source_uri.as_ref()
    }

    pub fn image_size(&self) -> [u32; 2] {
        [self.base_image.width(), self.base_image.height()]
    }

    pub fn add_overlay(&mut self, overlay: OverlayObject) {
        match overlay {
            OverlayObject::Crop(_) => {}
            annotation => self.overlays.push(annotation),
        }
    }

    pub fn set_overlay(&mut self, overlay_index: usize, overlay: OverlayObject) -> bool {
        match self.overlays.get_mut(overlay_index) {
            Some(slot) => {
                *slot = overlay;
                true
            }
            None => false,
        }
    }

    pub fn remove_overlay(&mut self, overlay_index: usize) -> bool {
        if overlay_index >= self.overlays.len() {
            return false;
        }

        self.overlays.remove(overlay_index);
        true
    }

    pub fn set_text_anchor(&mut self, overlay_index: usize, anchor: ImagePoint) -> bool {
        match self.overlays.get_mut(overlay_index) {
            Some(OverlayObject::Text(text)) => {
                text.anchor = anchor;
                true
            }
            _ => false,
        }
    }

    pub fn restore(
        &mut self,
        base_image: RgbaImage,
        overlays: Vec<OverlayObject>,
        source_uri: Option<String>,
    ) {
        self.base_image = base_image;
        self.overlays = overlays;
        self.source_uri = source_uri;
    }

    pub fn crop_to(&mut self, selection: ImageRect) -> AppResult<()> {
        let (selection, left, top, crop_width, crop_height) =
            self.selection_to_crop_bounds(selection)?;

        self.base_image =
            imageops::crop_imm(&self.base_image, left, top, crop_width, crop_height).to_image();
        self.overlays = self
            .overlays
            .iter()
            .filter(|overlay| overlay.bounds().intersects(selection))
            .map(|overlay| overlay.translated(-selection.min.x, -selection.min.y))
            .collect();

        Ok(())
    }

    pub fn base_image_region(&self, selection: Option<ImageRect>) -> AppResult<RgbaImage> {
        let Some(selection) = selection else {
            return Ok(self.base_image.clone());
        };

        let (_, left, top, crop_width, crop_height) = self.selection_to_crop_bounds(selection)?;
        Ok(imageops::crop_imm(&self.base_image, left, top, crop_width, crop_height).to_image())
    }

    pub fn render_flattened(&self) -> AppResult<RgbaImage> {
        let needs_font = self
            .overlays
            .iter()
            .any(|overlay| matches!(overlay, OverlayObject::Text(_)));
        let font = if needs_font {
            Some(load_export_font()?)
        } else {
            None
        };

        let mut flattened = self.base_image.clone();
        if self.overlays.is_empty() {
            return Ok(flattened);
        }

        let supersample = overlay_supersample_factor(flattened.width(), flattened.height());
        if supersample == 1 {
            for overlay in &self.overlays {
                render_overlay(&mut flattened, overlay, font.as_ref(), 1.0)?;
            }
            return Ok(flattened);
        }

        let Some(overlay_width) = flattened.width().checked_mul(supersample) else {
            for overlay in &self.overlays {
                render_overlay(&mut flattened, overlay, font.as_ref(), 1.0)?;
            }
            return Ok(flattened);
        };
        let Some(overlay_height) = flattened.height().checked_mul(supersample) else {
            for overlay in &self.overlays {
                render_overlay(&mut flattened, overlay, font.as_ref(), 1.0)?;
            }
            return Ok(flattened);
        };

        let mut overlay_layer = RgbaImage::new(overlay_width, overlay_height);
        for overlay in &self.overlays {
            render_overlay(
                &mut overlay_layer,
                overlay,
                font.as_ref(),
                supersample as f32,
            )?;
        }
        let overlay_layer = imageops::resize(
            &overlay_layer,
            flattened.width(),
            flattened.height(),
            imageops::FilterType::Lanczos3,
        );
        blend_overlay(&mut flattened, &overlay_layer);

        Ok(flattened)
    }

    pub fn save_png<P: AsRef<Path>>(&self, path: P) -> AppResult<()> {
        let image = self.render_flattened()?;
        image.save(path)?;
        Ok(())
    }

    fn selection_to_crop_bounds(
        &self,
        selection: ImageRect,
    ) -> AppResult<(ImageRect, u32, u32, u32, u32)> {
        let width = self.base_image.width() as f32;
        let height = self.base_image.height() as f32;
        let selection = selection.clamp_to_bounds(width, height);

        if selection.is_empty() {
            return Err(SnaptureError::Message("crop selection is empty".into()));
        }

        let left = selection.min.x.floor() as u32;
        let top = selection.min.y.floor() as u32;
        let crop_width = selection.width().ceil().max(1.0) as u32;
        let crop_height = selection.height().ceil().max(1.0) as u32;
        let crop_width = crop_width.min(self.base_image.width().saturating_sub(left));
        let crop_height = crop_height.min(self.base_image.height().saturating_sub(top));

        Ok((selection, left, top, crop_width, crop_height))
    }
}

fn render_overlay(
    image: &mut RgbaImage,
    overlay: &OverlayObject,
    font: Option<&FontArc>,
    scale: f32,
) -> AppResult<()> {
    match overlay {
        OverlayObject::Pen(stroke) => render_pen(image, stroke, scale),
        OverlayObject::Rectangle(rectangle) => render_rectangle(image, rectangle, scale),
        OverlayObject::Arrow(arrow) => render_arrow(image, arrow, scale),
        OverlayObject::Text(text) => render_text(image, text, font, scale)?,
        OverlayObject::Crop(_) => {}
    }

    Ok(())
}

fn render_pen(image: &mut RgbaImage, stroke: &PenStrokeOverlay, scale: f32) {
    let points: Vec<ImagePoint> = stroke
        .points
        .iter()
        .copied()
        .map(|point| scale_point(point, scale))
        .collect();
    render_polyline(
        image,
        &points,
        stroke.style.color.to_image(),
        stroke.style.thickness * scale,
    );
}

fn render_rectangle(image: &mut RgbaImage, rectangle: &RectangleOverlay, scale: f32) {
    let rect = scale_rect(rectangle.rect.normalized(), scale);
    let color = rectangle.style.color.to_image();
    let thickness = rectangle.style.thickness * scale;
    let top_left = rect.min;
    let top_right = ImagePoint::new(rect.max.x, rect.min.y);
    let bottom_left = ImagePoint::new(rect.min.x, rect.max.y);
    let bottom_right = rect.max;

    draw_thick_segment(image, top_left, top_right, color, thickness);
    draw_thick_segment(image, top_right, bottom_right, color, thickness);
    draw_thick_segment(image, bottom_right, bottom_left, color, thickness);
    draw_thick_segment(image, bottom_left, top_left, color, thickness);
}

fn render_arrow(image: &mut RgbaImage, arrow: &ArrowOverlay, scale: f32) {
    let start = scale_point(arrow.start, scale);
    let end = scale_point(arrow.end, scale);
    let color = arrow.style.color.to_image();
    let thickness = arrow.style.thickness * scale;
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let len = dx.hypot(dy).max(1.0);
    let ux = dx / len;
    let uy = dy / len;
    let head_len = (thickness * 5.0).max(14.0);
    let head_width = head_len * 0.45;
    let base = ImagePoint::new(end.x - ux * head_len, end.y - uy * head_len);
    let left = ImagePoint::new(base.x - uy * head_width, base.y + ux * head_width);
    let right = ImagePoint::new(base.x + uy * head_width, base.y - ux * head_width);

    draw_thick_segment(image, start, base, color, thickness);
    draw_thick_segment(image, end, left, color, thickness);
    draw_thick_segment(image, end, right, color, thickness);
}

fn render_text(
    image: &mut RgbaImage,
    text: &TextOverlay,
    font: Option<&FontArc>,
    scale: f32,
) -> AppResult<()> {
    let font = font.ok_or(SnaptureError::MissingFont)?;
    let anchor = scale_point(text.anchor, scale);
    let size = text.style.size * scale;
    let line_height = size * 1.25;
    let max_width = (image.width() as f32 - anchor.x).max(1.0);
    let wrapped_lines = wrap_text_for_export(&text.text, font, size, max_width);

    for (index, line) in wrapped_lines.iter().enumerate() {
        let y = anchor.y + line_height * index as f32;
        draw_text_mut(
            image,
            text.style.color.to_image(),
            anchor.x.round() as i32,
            y.round() as i32,
            size,
            font,
            line,
        );
    }

    Ok(())
}

fn wrap_text_for_export(text: &str, font: &FontArc, size: f32, max_width: f32) -> Vec<String> {
    let mut wrapped = Vec::new();
    let max_width = max_width.max(1.0);

    for raw_line in text.split('\n') {
        if raw_line.is_empty() {
            wrapped.push(String::new());
            continue;
        }

        let mut current = String::new();
        for ch in raw_line.chars() {
            let mut candidate = current.clone();
            candidate.push(ch);

            if !current.is_empty() && text_size(size, font, &candidate).0 as f32 > max_width {
                wrapped.push(current);
                current = ch.to_string();
            } else {
                current.push(ch);
            }
        }

        wrapped.push(current);
    }

    if wrapped.is_empty() {
        wrapped.push(String::new());
    }

    wrapped
}

fn render_polyline(
    image: &mut RgbaImage,
    points: &[ImagePoint],
    color: image::Rgba<u8>,
    thickness: f32,
) {
    if let Some(point) = points.first().copied() {
        draw_round_dot(image, point, color, thickness);
    }

    for window in points.windows(2) {
        let [start, end] = [window[0], window[1]];
        draw_thick_segment(image, start, end, color, thickness);
    }
}

fn draw_thick_segment(
    image: &mut RgbaImage,
    start: ImagePoint,
    end: ImagePoint,
    color: image::Rgba<u8>,
    thickness: f32,
) {
    if thickness <= 1.5 {
        draw_antialiased_line_segment_mut(
            image,
            (start.x.round() as i32, start.y.round() as i32),
            (end.x.round() as i32, end.y.round() as i32),
            color,
            interpolate,
        );
        return;
    }

    let polygon = capsule_polygon_points(start, end, thickness / 2.0);
    if polygon.len() >= 3 {
        draw_antialiased_polygon_mut(image, &polygon, color, interpolate);
    } else {
        draw_antialiased_line_segment_mut(
            image,
            (start.x.round() as i32, start.y.round() as i32),
            (end.x.round() as i32, end.y.round() as i32),
            color,
            interpolate,
        );
    }
}

fn draw_round_dot(
    image: &mut RgbaImage,
    center: ImagePoint,
    color: image::Rgba<u8>,
    thickness: f32,
) {
    let polygon = circle_polygon_points(center, (thickness / 2.0).max(0.75));
    if polygon.len() >= 3 {
        draw_antialiased_polygon_mut(image, &polygon, color, interpolate);
        return;
    }

    let x = center.x.round() as i32;
    let y = center.y.round() as i32;
    if x >= 0 && y >= 0 && x < image.width() as i32 && y < image.height() as i32 {
        image.put_pixel(x as u32, y as u32, color);
    }
}

fn capsule_polygon_points(start: ImagePoint, end: ImagePoint, radius: f32) -> Vec<Point<i32>> {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length = dx.hypot(dy);
    if length < 0.5 {
        return circle_polygon_points(start, radius);
    }

    let base_angle = dy.atan2(dx);
    let arc_steps = arc_steps(radius);
    let mut points = Vec::with_capacity((arc_steps + 1) * 2);

    for step in 0..=arc_steps {
        let angle = base_angle + PI * 0.5 + PI * step as f32 / arc_steps as f32;
        push_unique_point(&mut points, point_on_circle(start, radius, angle));
    }

    for step in 0..=arc_steps {
        let angle = base_angle - PI * 0.5 + PI * step as f32 / arc_steps as f32;
        push_unique_point(&mut points, point_on_circle(end, radius, angle));
    }

    if points.first() == points.last() {
        points.pop();
    }

    points
}

fn circle_polygon_points(center: ImagePoint, radius: f32) -> Vec<Point<i32>> {
    let steps = (arc_steps(radius) * 2).max(10);
    let mut points = Vec::with_capacity(steps);

    for step in 0..steps {
        let angle = 2.0 * PI * step as f32 / steps as f32;
        push_unique_point(&mut points, point_on_circle(center, radius, angle));
    }

    if points.first() == points.last() {
        points.pop();
    }

    points
}

fn point_on_circle(center: ImagePoint, radius: f32, angle: f32) -> Point<i32> {
    Point::new(
        (center.x + radius * angle.cos()).round() as i32,
        (center.y + radius * angle.sin()).round() as i32,
    )
}

fn push_unique_point(points: &mut Vec<Point<i32>>, point: Point<i32>) {
    if points.last().copied() != Some(point) {
        points.push(point);
    }
}

fn arc_steps(radius: f32) -> usize {
    ((radius * 2.0).ceil() as usize).clamp(6, 18)
}

fn scale_point(point: ImagePoint, scale: f32) -> ImagePoint {
    ImagePoint::new(point.x * scale, point.y * scale)
}

fn scale_rect(rect: ImageRect, scale: f32) -> ImageRect {
    ImageRect::from_points(scale_point(rect.min, scale), scale_point(rect.max, scale))
}

fn overlay_supersample_factor(width: u32, height: u32) -> u32 {
    let pixels = u64::from(width) * u64::from(height);
    if pixels <= EXPORT_SUPERSAMPLE_3X_MAX_PIXELS {
        3
    } else if pixels <= EXPORT_SUPERSAMPLE_2X_MAX_PIXELS {
        2
    } else {
        1
    }
}

fn blend_overlay(base: &mut RgbaImage, overlay: &RgbaImage) {
    for (x, y, overlay_pixel) in overlay.enumerate_pixels() {
        if overlay_pixel.0[3] == 0 {
            continue;
        }

        let mut base_pixel = *base.get_pixel(x, y);
        base_pixel.blend(overlay_pixel);
        base.put_pixel(x, y, base_pixel);
    }
}

fn load_export_font() -> AppResult<FontArc> {
    let mut db = Database::new();
    db.load_system_fonts();

    let families = [Family::SansSerif];
    let query = Query {
        families: &families,
        ..Query::default()
    };

    let id = db.query(&query).ok_or(SnaptureError::MissingFont)?;
    let face = db.face(id).ok_or(SnaptureError::MissingFont)?;
    let bytes = match &face.source {
        Source::Binary(data) => data.as_ref().as_ref().to_vec(),
        Source::File(path) => fs::read(path)?,
        Source::SharedFile(path, _) => fs::read(path)?,
    };

    FontArc::try_from_vec(bytes).map_err(|_| SnaptureError::MissingFont)
}
