use crate::model::{
    overlay::{OverlayObject, RectangleOverlay},
    types::{ImagePoint, ImageRect, StrokeStyle},
};

#[derive(Clone, Debug)]
pub struct RectangleDraft {
    start: ImagePoint,
    current: ImagePoint,
    style: StrokeStyle,
}

impl RectangleDraft {
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
        OverlayObject::Rectangle(RectangleOverlay {
            rect: ImageRect::from_points(self.start, self.current),
            style: self.style.clone(),
        })
    }

    pub fn finish(self) -> Option<OverlayObject> {
        let rect = ImageRect::from_points(self.start, self.current);
        if rect.is_empty() {
            return None;
        }

        Some(OverlayObject::Rectangle(RectangleOverlay {
            rect,
            style: self.style,
        }))
    }
}
