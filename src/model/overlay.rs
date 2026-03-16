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

    pub fn transformed_to_bounds(&self, from_bounds: ImageRect, to_bounds: ImageRect) -> Self {
        let from_bounds = from_bounds.normalized();
        let to_bounds = to_bounds.normalized();
        let uniform_scale = uniform_scale_factor(from_bounds, to_bounds);

        match self {
            Self::Pen(stroke) => Self::Pen(PenStrokeOverlay {
                points: stroke
                    .points
                    .iter()
                    .map(|point| map_point_between_rects(*point, from_bounds, to_bounds))
                    .collect(),
                style: StrokeStyle {
                    color: stroke.style.color,
                    thickness: (stroke.style.thickness * uniform_scale).max(1.0),
                },
            }),
            Self::Rectangle(rectangle) => Self::Rectangle(RectangleOverlay {
                rect: ImageRect::from_points(
                    map_point_between_rects(rectangle.rect.min, from_bounds, to_bounds),
                    map_point_between_rects(rectangle.rect.max, from_bounds, to_bounds),
                ),
                style: StrokeStyle {
                    color: rectangle.style.color,
                    thickness: (rectangle.style.thickness * uniform_scale).max(1.0),
                },
            }),
            Self::Arrow(arrow) => Self::Arrow(ArrowOverlay {
                start: map_point_between_rects(arrow.start, from_bounds, to_bounds),
                end: map_point_between_rects(arrow.end, from_bounds, to_bounds),
                style: StrokeStyle {
                    color: arrow.style.color,
                    thickness: (arrow.style.thickness * uniform_scale).max(1.0),
                },
            }),
            Self::Text(text) => Self::Text(TextOverlay {
                anchor: map_point_between_rects(text.anchor, from_bounds, to_bounds),
                text: text.text.clone(),
                style: TextStyle {
                    color: text.style.color,
                    size: (text.style.size * uniform_scale).max(8.0),
                },
            }),
            Self::Crop(crop) => Self::Crop(CropOverlay {
                rect: ImageRect::from_points(
                    map_point_between_rects(crop.rect.min, from_bounds, to_bounds),
                    map_point_between_rects(crop.rect.max, from_bounds, to_bounds),
                ),
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

fn map_point_between_rects(point: ImagePoint, from: ImageRect, to: ImageRect) -> ImagePoint {
    ImagePoint::new(
        map_axis_between_ranges(point.x, from.min.x, from.width(), to.min.x, to.width()),
        map_axis_between_ranges(point.y, from.min.y, from.height(), to.min.y, to.height()),
    )
}

fn map_axis_between_ranges(
    value: f32,
    from_start: f32,
    from_size: f32,
    to_start: f32,
    to_size: f32,
) -> f32 {
    if from_size.abs() <= f32::EPSILON {
        return to_start + to_size * 0.5;
    }

    let normalized = (value - from_start) / from_size;
    to_start + normalized * to_size
}

fn uniform_scale_factor(from: ImageRect, to: ImageRect) -> f32 {
    let scale_x = if from.width().abs() <= f32::EPSILON {
        1.0
    } else {
        (to.width() / from.width()).abs()
    };
    let scale_y = if from.height().abs() <= f32::EPSILON {
        1.0
    } else {
        (to.height() / from.height()).abs()
    };

    ((scale_x + scale_y) * 0.5).max(0.1)
}
