use std::path::PathBuf;

use ashpd::desktop::screenshot::Screenshot;
use image::RgbaImage;
use url::Url;

use crate::error::{AppResult, SnaptureError};

pub struct CapturedImage {
    pub image: RgbaImage,
    pub source_uri: Option<String>,
}

pub async fn capture_screenshot() -> AppResult<CapturedImage> {
    let request = Screenshot::request().modal(true).interactive(true);
    let response = request.send().await?;
    let screenshot = response.response()?;
    let uri = screenshot.uri().to_string();
    let path = file_path_from_uri(&uri)?;
    let image = image::open(path)?.to_rgba8();

    Ok(CapturedImage {
        image,
        source_uri: Some(uri),
    })
}

fn file_path_from_uri(uri: &str) -> AppResult<PathBuf> {
    let url = Url::parse(uri)?;
    url.to_file_path()
        .map_err(|_| SnaptureError::InvalidUri(uri.to_owned()))
}
