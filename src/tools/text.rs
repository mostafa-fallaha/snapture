use crate::model::{
    overlay::{OverlayObject, TextOverlay},
    types::{ImagePoint, TextStyle},
};

pub fn build_text_overlay(
    anchor: ImagePoint,
    text: impl Into<String>,
    style: TextStyle,
) -> Option<OverlayObject> {
    let text = text.into();
    if text.trim().is_empty() {
        return None;
    }

    Some(OverlayObject::Text(TextOverlay {
        anchor,
        text,
        style,
    }))
}
