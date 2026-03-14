use std::path::{Path, PathBuf};

use crate::{editor::document::Document, error::AppResult};

pub fn save_document_png(document: &Document, path: impl AsRef<Path>) -> AppResult<PathBuf> {
    let path = path.as_ref().to_path_buf();
    document.save_png(&path)?;
    Ok(path)
}
