use eframe::egui::Color32;
use image::Rgba;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImagePoint {
    pub x: f32,
    pub y: f32,
}

impl ImagePoint {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn translated(self, dx: f32, dy: f32) -> Self {
        Self::new(self.x + dx, self.y + dy)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageRect {
    pub min: ImagePoint,
    pub max: ImagePoint,
}

impl ImageRect {
    pub fn from_points(a: ImagePoint, b: ImagePoint) -> Self {
        Self { min: a, max: b }.normalized()
    }

    pub fn normalized(self) -> Self {
        Self {
            min: ImagePoint::new(self.min.x.min(self.max.x), self.min.y.min(self.max.y)),
            max: ImagePoint::new(self.min.x.max(self.max.x), self.min.y.max(self.max.y)),
        }
    }

    pub fn width(self) -> f32 {
        self.max.x - self.min.x
    }

    pub fn height(self) -> f32 {
        self.max.y - self.min.y
    }

    pub fn is_empty(self) -> bool {
        self.width() < 1.0 || self.height() < 1.0
    }

    pub fn translated(self, dx: f32, dy: f32) -> Self {
        Self {
            min: self.min.translated(dx, dy),
            max: self.max.translated(dx, dy),
        }
    }

    pub fn clamp_to_bounds(self, width: f32, height: f32) -> Self {
        let rect = self.normalized();
        Self {
            min: ImagePoint::new(rect.min.x.clamp(0.0, width), rect.min.y.clamp(0.0, height)),
            max: ImagePoint::new(rect.max.x.clamp(0.0, width), rect.max.y.clamp(0.0, height)),
        }
    }

    pub fn contains(self, point: ImagePoint) -> bool {
        let rect = self.normalized();
        point.x >= rect.min.x
            && point.x <= rect.max.x
            && point.y >= rect.min.y
            && point.y <= rect.max.y
    }

    pub fn intersects(self, other: Self) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RgbaColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl RgbaColor {
    pub const fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_egui(self) -> Color32 {
        Color32::from_rgba_unmultiplied(self.r, self.g, self.b, self.a)
    }

    pub fn to_image(self) -> Rgba<u8> {
        Rgba([self.r, self.g, self.b, self.a])
    }
}

impl Default for RgbaColor {
    fn default() -> Self {
        Self::from_rgba(255, 59, 48, 255)
    }
}

impl From<Color32> for RgbaColor {
    fn from(value: Color32) -> Self {
        Self::from_rgba(value.r(), value.g(), value.b(), value.a())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrokeStyle {
    pub color: RgbaColor,
    pub thickness: f32,
}

impl StrokeStyle {
    pub fn new(color: RgbaColor, thickness: f32) -> Self {
        Self { color, thickness }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub color: RgbaColor,
    pub size: f32,
}

impl TextStyle {
    pub fn new(color: RgbaColor, size: f32) -> Self {
        Self { color, size }
    }
}
