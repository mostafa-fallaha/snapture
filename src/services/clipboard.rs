use std::borrow::Cow;

use arboard::{Clipboard, ImageData};
use image::RgbaImage;

use crate::{editor::document::Document, error::AppResult};

pub fn copy_rgba_image(image: &RgbaImage) -> AppResult<()> {
    copy_raw_image(
        image.width() as usize,
        image.height() as usize,
        image.as_raw().clone(),
    )
}

pub fn copy_document_image(document: &Document) -> AppResult<()> {
    let rendered = document.render_flattened()?;
    copy_raw_image(
        rendered.width() as usize,
        rendered.height() as usize,
        rendered.into_raw(),
    )
}

fn copy_raw_image(width: usize, height: usize, bytes: Vec<u8>) -> AppResult<()> {
    let image = ImageData {
        width,
        height,
        bytes: Cow::Owned(bytes),
    };

    let mut clipboard = Clipboard::new()?;
    clipboard.set_image(image)?;
    Ok(())
}
