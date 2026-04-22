use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

use image::RgbaImage;
use url::Url;

use crate::{
    capture::capture_before_ui,
    error::{AppResult, SnaptureError},
    services::clipboard,
};

pub struct LaunchImage {
    pub image: RgbaImage,
    pub source_uri: Option<String>,
    pub initial_save_path: PathBuf,
}

#[derive(Debug, Eq, PartialEq)]
enum LaunchRequest {
    CaptureToClipboard,
    OpenFile(PathBuf),
}

pub enum LaunchAction {
    Exit,
    OpenEditor(LaunchImage),
}

pub fn launch_before_ui() -> AppResult<LaunchAction> {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    let request = resolve_launch_request(args)?;

    match request {
        LaunchRequest::CaptureToClipboard => capture_to_clipboard().map(|()| LaunchAction::Exit),
        LaunchRequest::OpenFile(path) => open_png(path).map(LaunchAction::OpenEditor),
    }
}

fn resolve_launch_request(args: Vec<OsString>) -> AppResult<LaunchRequest> {
    match args.len() {
        0 => Ok(LaunchRequest::CaptureToClipboard),
        1 => Ok(LaunchRequest::OpenFile(PathBuf::from(&args[0]))),
        count => Err(SnaptureError::Message(format!(
            "expected exactly zero or one PNG file path, got {count}"
        ))),
    }
}

fn capture_to_clipboard() -> AppResult<()> {
    let capture = capture_before_ui()
        .map_err(|error| SnaptureError::Message(format!("screenshot capture failed: {error}")))?;

    clipboard::copy_rgba_image(&capture.image).map_err(|error| {
        SnaptureError::Message(format!("failed to copy screenshot to clipboard: {error}"))
    })?;

    Ok(())
}

fn open_png(path: PathBuf) -> AppResult<LaunchImage> {
    let path = normalize_png_path(path)?;
    let image = image::open(&path)
        .map_err(|error| {
            SnaptureError::Message(format!("failed to open PNG {}: {error}", path.display()))
        })?
        .to_rgba8();
    let source_uri = Some(file_uri_from_path(&path)?);

    Ok(LaunchImage {
        image,
        source_uri,
        initial_save_path: path,
    })
}

fn normalize_png_path(path: PathBuf) -> AppResult<PathBuf> {
    if !has_png_extension(&path) {
        return Err(SnaptureError::Message(format!(
            "expected a PNG file path, got {}",
            path.display()
        )));
    }

    let path = absolutize_path(path)?;
    if !path.is_file() {
        return Err(SnaptureError::Message(format!(
            "PNG file not found: {}",
            path.display()
        )));
    }

    Ok(path)
}

fn has_png_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("png"))
        .unwrap_or(false)
}

fn absolutize_path(path: PathBuf) -> AppResult<PathBuf> {
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(env::current_dir()?.join(path))
    }
}

fn file_uri_from_path(path: &Path) -> AppResult<String> {
    Url::from_file_path(path)
        .map(|url| url.to_string())
        .map_err(|_| SnaptureError::InvalidUri(path.display().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn resolves_capture_to_clipboard_when_no_path_is_passed() {
        let request = resolve_launch_request(Vec::<OsString>::new()).unwrap();
        assert_eq!(request, LaunchRequest::CaptureToClipboard);
    }

    #[test]
    fn resolves_single_path_when_one_argument_is_passed() {
        let request = resolve_launch_request(vec![OsString::from("sample.png")]).unwrap();
        assert_eq!(
            request,
            LaunchRequest::OpenFile(PathBuf::from("sample.png"))
        );
    }

    #[test]
    fn rejects_multiple_paths() {
        let error = resolve_launch_request(vec![
            OsString::from("first.png"),
            OsString::from("second.png"),
        ])
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "expected exactly zero or one PNG file path, got 2"
        );
    }

    #[test]
    fn rejects_non_png_paths() {
        let error = normalize_png_path(PathBuf::from("sample.jpg")).unwrap_err();
        assert_eq!(
            error.to_string(),
            "expected a PNG file path, got sample.jpg"
        );
    }

    #[test]
    fn accepts_existing_png_paths() {
        let temp_path = unique_temp_path("snapture-launch-test", "png");
        fs::write(&temp_path, []).unwrap();

        let normalized = normalize_png_path(temp_path.clone()).unwrap();
        assert_eq!(normalized, temp_path);

        fs::remove_file(temp_path).unwrap();
    }

    fn unique_temp_path(prefix: &str, extension: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("{prefix}-{timestamp}.{extension}"))
    }
}
