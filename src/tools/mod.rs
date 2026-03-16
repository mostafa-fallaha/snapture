pub mod arrow;
pub mod crop;
pub mod highlighter;
pub mod pen;
pub mod rect;
pub mod text;

use crate::{
    model::{
        overlay::{CropOverlay, OverlayObject},
        types::{ImagePoint, StrokeStyle},
    },
    tools::{
        arrow::ArrowDraft, crop::CropDraft, highlighter::HighlighterDraft, pen::PenDraft,
        rect::RectangleDraft,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolKind {
    Select,
    Pen,
    Highlighter,
    Rectangle,
    Arrow,
    Text,
    Crop,
}

impl ToolKind {
    pub const ALL: [Self; 7] = [
        Self::Select,
        Self::Pen,
        Self::Highlighter,
        Self::Rectangle,
        Self::Arrow,
        Self::Text,
        Self::Crop,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Pen => "Pen",
            Self::Highlighter => "Highlight",
            Self::Rectangle => "Rect",
            Self::Arrow => "Arrow",
            Self::Text => "Text",
            Self::Crop => "Crop",
        }
    }
}

#[derive(Clone, Debug)]
pub enum DraftOverlay {
    Pen(PenDraft),
    Highlighter(HighlighterDraft),
    Rectangle(RectangleDraft),
    Arrow(ArrowDraft),
    Crop(CropDraft),
}

impl DraftOverlay {
    pub fn update(&mut self, point: ImagePoint) {
        match self {
            Self::Pen(draft) => draft.push(point),
            Self::Highlighter(draft) => draft.update(point),
            Self::Rectangle(draft) => draft.update(point),
            Self::Arrow(draft) => draft.update(point),
            Self::Crop(draft) => draft.update(point),
        }
    }

    pub fn preview(&self) -> OverlayObject {
        match self {
            Self::Pen(draft) => draft.preview(),
            Self::Highlighter(draft) => draft.preview(),
            Self::Rectangle(draft) => draft.preview(),
            Self::Arrow(draft) => draft.preview(),
            Self::Crop(draft) => OverlayObject::Crop(CropOverlay { rect: draft.rect() }),
        }
    }

    pub fn finish(self) -> Option<OverlayObject> {
        match self {
            Self::Pen(draft) => draft.finish(),
            Self::Highlighter(draft) => draft.finish(),
            Self::Rectangle(draft) => draft.finish(),
            Self::Arrow(draft) => draft.finish(),
            Self::Crop(draft) => Some(OverlayObject::Crop(CropOverlay { rect: draft.rect() })),
        }
    }
}

pub fn begin_drag(tool: ToolKind, start: ImagePoint, style: StrokeStyle) -> Option<DraftOverlay> {
    match tool {
        ToolKind::Select => None,
        ToolKind::Pen => Some(DraftOverlay::Pen(PenDraft::new(start, style))),
        ToolKind::Highlighter => Some(DraftOverlay::Highlighter(HighlighterDraft::new(
            start, style,
        ))),
        ToolKind::Rectangle => Some(DraftOverlay::Rectangle(RectangleDraft::new(start, style))),
        ToolKind::Arrow => Some(DraftOverlay::Arrow(ArrowDraft::new(start, style))),
        ToolKind::Crop => Some(DraftOverlay::Crop(CropDraft::new(start))),
        ToolKind::Text => None,
    }
}
