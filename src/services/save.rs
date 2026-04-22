use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::{editor::document::Document, error::AppResult};

pub fn choose_save_path(default_path: impl AsRef<Path>) -> AppResult<Option<PathBuf>> {
    let default_path = default_path.as_ref();
    let output = Command::new("zenity")
        .arg("--file-selection")
        .arg("--save")
        .arg("--confirm-overwrite")
        .arg("--title=Save Image")
        .arg(format!("--filename={}", default_path.display()))
        .output()?;

    if output.status.success() {
        let selected = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if selected.is_empty() {
            return Ok(None);
        }

        return Ok(Some(ensure_png_extension(PathBuf::from(selected))));
    }

    match output.status.code() {
        Some(1) => Ok(None),
        Some(code) => Err(std::io::Error::other(format!(
            "zenity save dialog failed with exit code {code}"
        ))
        .into()),
        None => Err(std::io::Error::other("zenity save dialog terminated unexpectedly").into()),
    }
}

pub fn save_document_png(document: &Document, path: impl AsRef<Path>) -> AppResult<PathBuf> {
    let path = path.as_ref().to_path_buf();
    document.save_png(&path)?;
    Ok(path)
}

fn ensure_png_extension(mut path: PathBuf) -> PathBuf {
    let needs_png_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| !extension.eq_ignore_ascii_case("png"))
        .unwrap_or(true);

    if needs_png_extension {
        path.set_extension("png");
    }

    path
}
