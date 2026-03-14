use crate::model::types::{ImagePoint, ImageRect};

#[derive(Clone, Debug)]
pub struct CropDraft {
    start: ImagePoint,
    current: ImagePoint,
}

impl CropDraft {
    pub fn new(start: ImagePoint) -> Self {
        Self {
            start,
            current: start,
        }
    }

    pub fn update(&mut self, point: ImagePoint) {
        self.current = point;
    }

    pub fn rect(&self) -> ImageRect {
        ImageRect::from_points(self.start, self.current)
    }
}
