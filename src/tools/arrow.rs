use crate::model::{
    overlay::{ArrowOverlay, OverlayObject},
    types::{ImagePoint, StrokeStyle},
};

#[derive(Clone, Debug)]
pub struct ArrowDraft {
    start: ImagePoint,
    current: ImagePoint,
    style: StrokeStyle,
}

impl ArrowDraft {
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
        OverlayObject::Arrow(ArrowOverlay {
            start: self.start,
            end: self.current,
            style: self.style.clone(),
        })
    }

    pub fn finish(self) -> Option<OverlayObject> {
        let dx = self.current.x - self.start.x;
        let dy = self.current.y - self.start.y;
        if dx * dx + dy * dy < 1.0 {
            return None;
        }

        Some(OverlayObject::Arrow(ArrowOverlay {
            start: self.start,
            end: self.current,
            style: self.style,
        }))
    }
}
