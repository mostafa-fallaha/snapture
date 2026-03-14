use crate::model::types::{ImagePoint, ImageRect, StrokeStyle, TextStyle};

#[derive(Clone, Debug, PartialEq)]
pub struct PenStrokeOverlay {
    pub points: Vec<ImagePoint>,
    pub style: StrokeStyle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RectangleOverlay {
    pub rect: ImageRect,
    pub style: StrokeStyle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrowOverlay {
    pub start: ImagePoint,
    pub end: ImagePoint,
    pub style: StrokeStyle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextOverlay {
    pub anchor: ImagePoint,
    pub text: String,
    pub style: TextStyle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CropOverlay {
    pub rect: ImageRect,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OverlayObject {
    Pen(PenStrokeOverlay),
    Rectangle(RectangleOverlay),
    Arrow(ArrowOverlay),
    Text(TextOverlay),
    Crop(CropOverlay),
}

impl OverlayObject {
    pub fn bounds(&self) -> ImageRect {
        match self {
            Self::Pen(stroke) => bounds_from_points(&stroke.points, stroke.style.thickness),
            Self::Rectangle(rect) => rect.rect,
            Self::Arrow(arrow) => {
                bounds_from_points(&[arrow.start, arrow.end], arrow.style.thickness * 3.0)
            }
            Self::Text(text) => {
                let lines: Vec<&str> = text.text.lines().collect();
                let line_count = lines.len().max(1) as f32;
                let max_chars = lines
                    .iter()
                    .map(|line| line.chars().count())
                    .max()
                    .unwrap_or(1) as f32;
                let width = max_chars * text.style.size * 0.65;
                let height = line_count * text.style.size * 1.3;

                ImageRect::from_points(
                    text.anchor,
                    ImagePoint::new(text.anchor.x + width, text.anchor.y + height),
                )
            }
            Self::Crop(crop) => crop.rect,
        }
    }

    pub fn translated(&self, dx: f32, dy: f32) -> Self {
        match self {
            Self::Pen(stroke) => Self::Pen(PenStrokeOverlay {
                points: stroke
                    .points
                    .iter()
                    .map(|point| point.translated(dx, dy))
                    .collect(),
                style: stroke.style.clone(),
            }),
            Self::Rectangle(rectangle) => Self::Rectangle(RectangleOverlay {
                rect: rectangle.rect.translated(dx, dy),
                style: rectangle.style.clone(),
            }),
            Self::Arrow(arrow) => Self::Arrow(ArrowOverlay {
                start: arrow.start.translated(dx, dy),
                end: arrow.end.translated(dx, dy),
                style: arrow.style.clone(),
            }),
            Self::Text(text) => Self::Text(TextOverlay {
                anchor: text.anchor.translated(dx, dy),
                text: text.text.clone(),
                style: text.style.clone(),
            }),
            Self::Crop(crop) => Self::Crop(CropOverlay {
                rect: crop.rect.translated(dx, dy),
            }),
        }
    }
}

fn bounds_from_points(points: &[ImagePoint], padding: f32) -> ImageRect {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    for point in points {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }

    if points.is_empty() {
        return ImageRect::from_points(ImagePoint::new(0.0, 0.0), ImagePoint::new(0.0, 0.0));
    }

    ImageRect::from_points(
        ImagePoint::new(min_x - padding, min_y - padding),
        ImagePoint::new(max_x + padding, max_y + padding),
    )
}
