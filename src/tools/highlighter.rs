use crate::{
    model::{
        overlay::OverlayObject,
        types::{ImagePoint, StrokeStyle},
    },
    tools::pen::PenDraft,
};

#[derive(Clone, Debug)]
pub struct HighlighterDraft {
    inner: PenDraft,
}

impl HighlighterDraft {
    pub fn new(start: ImagePoint, style: StrokeStyle) -> Self {
        Self {
            inner: PenDraft::new(start, style),
        }
    }

    pub fn update(&mut self, point: ImagePoint) {
        self.inner.push(point);
    }

    pub fn preview(&self) -> OverlayObject {
        self.inner.preview()
    }

    pub fn finish(self) -> Option<OverlayObject> {
        self.inner.finish()
    }
}
