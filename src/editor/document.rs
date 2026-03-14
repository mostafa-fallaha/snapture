use std::{fs, path::Path};

use ab_glyph::FontArc;
use fontdb::{Database, Family, Query, Source};
use image::{RgbaImage, imageops};
use imageproc::drawing::{draw_filled_circle_mut, draw_text_mut};

use crate::{
    error::{AppResult, SnaptureError},
    model::{
        overlay::{ArrowOverlay, OverlayObject, PenStrokeOverlay, RectangleOverlay, TextOverlay},
        types::{ImagePoint, ImageRect},
    },
};

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
        for overlay in &self.overlays {
            render_overlay(&mut flattened, overlay, font.as_ref())?;
        }

        Ok(flattened)
    }

    pub fn save_png<P: AsRef<Path>>(&self, path: P) -> AppResult<()> {
        let image = self.render_flattened()?;
        image.save(path)?;
        Ok(())
    }
}

fn render_overlay(
    image: &mut RgbaImage,
    overlay: &OverlayObject,
    font: Option<&FontArc>,
) -> AppResult<()> {
    match overlay {
        OverlayObject::Pen(stroke) => render_pen(image, stroke),
        OverlayObject::Rectangle(rectangle) => render_rectangle(image, rectangle),
        OverlayObject::Arrow(arrow) => render_arrow(image, arrow),
        OverlayObject::Text(text) => render_text(image, text, font)?,
        OverlayObject::Crop(_) => {}
    }

    Ok(())
}

fn render_pen(image: &mut RgbaImage, stroke: &PenStrokeOverlay) {
    render_polyline(
        image,
        &stroke.points,
        stroke.style.color.to_image(),
        stroke.style.thickness,
    );
}

fn render_rectangle(image: &mut RgbaImage, rectangle: &RectangleOverlay) {
    let rect = rectangle.rect.normalized();
    let color = rectangle.style.color.to_image();
    let thickness = rectangle.style.thickness;
    let top_left = rect.min;
    let top_right = ImagePoint::new(rect.max.x, rect.min.y);
    let bottom_left = ImagePoint::new(rect.min.x, rect.max.y);
    let bottom_right = rect.max;

    draw_thick_segment(image, top_left, top_right, color, thickness);
    draw_thick_segment(image, top_right, bottom_right, color, thickness);
    draw_thick_segment(image, bottom_right, bottom_left, color, thickness);
    draw_thick_segment(image, bottom_left, top_left, color, thickness);
}

fn render_arrow(image: &mut RgbaImage, arrow: &ArrowOverlay) {
    let color = arrow.style.color.to_image();
    let thickness = arrow.style.thickness;
    let dx = arrow.end.x - arrow.start.x;
    let dy = arrow.end.y - arrow.start.y;
    let len = dx.hypot(dy).max(1.0);
    let ux = dx / len;
    let uy = dy / len;
    let head_len = (thickness * 5.0).max(14.0);
    let head_width = head_len * 0.45;
    let base = ImagePoint::new(arrow.end.x - ux * head_len, arrow.end.y - uy * head_len);
    let left = ImagePoint::new(base.x - uy * head_width, base.y + ux * head_width);
    let right = ImagePoint::new(base.x + uy * head_width, base.y - ux * head_width);

    draw_thick_segment(image, arrow.start, base, color, thickness);
    draw_thick_segment(image, arrow.end, left, color, thickness);
    draw_thick_segment(image, arrow.end, right, color, thickness);
}

fn render_text(image: &mut RgbaImage, text: &TextOverlay, font: Option<&FontArc>) -> AppResult<()> {
    let font = font.ok_or(SnaptureError::MissingFont)?;
    let line_height = text.style.size * 1.25;

    for (index, line) in text.text.lines().enumerate() {
        let y = text.anchor.y + line_height * index as f32;
        draw_text_mut(
            image,
            text.style.color.to_image(),
            text.anchor.x.round() as i32,
            y.round() as i32,
            text.style.size,
            font,
            line,
        );
    }

    Ok(())
}

fn render_polyline(
    image: &mut RgbaImage,
    points: &[ImagePoint],
    color: image::Rgba<u8>,
    thickness: f32,
) {
    if let Some(point) = points.first().copied() {
        let radius = (thickness / 2.0).ceil().max(1.0) as i32;
        draw_filled_circle_mut(
            image,
            (point.x.round() as i32, point.y.round() as i32),
            radius,
            color,
        );
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
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let steps = dx.hypot(dy).ceil().max(1.0) as usize;
    let radius = (thickness / 2.0).ceil().max(1.0) as i32;

    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let x = start.x + dx * t;
        let y = start.y + dy * t;
        draw_filled_circle_mut(image, (x.round() as i32, y.round() as i32), radius, color);
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
