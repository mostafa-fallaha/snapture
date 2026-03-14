use crate::model::{
    overlay::{OverlayObject, PenStrokeOverlay},
    types::{ImagePoint, StrokeStyle},
};

#[derive(Clone, Debug)]
pub struct PenDraft {
    points: Vec<ImagePoint>,
    style: StrokeStyle,
}

impl PenDraft {
    pub fn new(start: ImagePoint, style: StrokeStyle) -> Self {
        Self {
            points: vec![start],
            style,
        }
    }

    pub fn push(&mut self, point: ImagePoint) {
        let should_push = self
            .points
            .last()
            .map(|last| {
                let dx = point.x - last.x;
                let dy = point.y - last.y;
                dx * dx + dy * dy > 0.25
            })
            .unwrap_or(true);

        if should_push {
            self.points.push(point);
        }
    }

    pub fn preview(&self) -> OverlayObject {
        OverlayObject::Pen(PenStrokeOverlay {
            points: self.points.clone(),
            style: self.style.clone(),
        })
    }

    pub fn finish(self) -> Option<OverlayObject> {
        if self.points.is_empty() {
            return None;
        }

        Some(self.preview())
    }
}
