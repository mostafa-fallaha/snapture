use std::borrow::Cow;

use arboard::{Clipboard, ImageData};

use crate::{editor::document::Document, error::AppResult};

pub fn copy_document_image(document: &Document) -> AppResult<()> {
    let rendered = document.render_flattened()?;
    let width = rendered.width() as usize;
    let height = rendered.height() as usize;
    let bytes = rendered.into_raw();
    let image = ImageData {
        width,
        height,
        bytes: Cow::Owned(bytes),
    };

    let mut clipboard = Clipboard::new()?;
    clipboard.set_image(image)?;
    Ok(())
}
