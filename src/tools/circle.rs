use crate::model::{
    overlay::{CircleOverlay, OverlayObject},
    types::{ImagePoint, StrokeStyle},
};

#[derive(Clone, Debug)]
pub struct CircleDraft {
    start: ImagePoint,
    current: ImagePoint,
    style: StrokeStyle,
}

impl CircleDraft {
    pub fn new(start: ImagePoint, style: StrokeStyle) -> Self {
        Self {
            start,
            current: start,
            style,
        }
    }

    pub fn update(&mut self, point: ImagePoint) {
        self.current = point;
    }

    pub fn preview(&self) -> OverlayObject {
        let (center, radius) = self.geometry();
        OverlayObject::Circle(CircleOverlay {
            center,
            radius,
            style: self.style.clone(),
        })
    }

    pub fn finish(self) -> Option<OverlayObject> {
        let (center, radius) = self.geometry();
        if radius < 1.0 {
            return None;
        }

        Some(OverlayObject::Circle(CircleOverlay {
            center,
            radius,
            style: self.style,
        }))
    }

    fn geometry(&self) -> (ImagePoint, f32) {
        let center = ImagePoint::new(
            (self.start.x + self.current.x) * 0.5,
            (self.start.y + self.current.y) * 0.5,
        );
        let radius = (self.current.x - self.start.x).hypot(self.current.y - self.start.y) * 0.5;
        (center, radius)
    }
}
