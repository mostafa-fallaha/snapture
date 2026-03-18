use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use image::{DynamicImage, RgbImage, RgbaImage};
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;

use crate::error::{AppResult, SnaptureError};

const DETECTION_MODEL_FILE: &str = "text-detection.rten";
const RECOGNITION_MODEL_FILE: &str = "text-recognition.rten";
const DARK_IMAGE_LUMINANCE_THRESHOLD: f32 = 110.0;

pub fn extract_text(image: RgbaImage) -> AppResult<String> {
    extract_text_inner(image).map_err(SnaptureError::from)
}

fn extract_text_inner(image: RgbaImage) -> Result<String> {
    let engine = load_engine()?;
    let image = preprocess_image(DynamicImage::ImageRgba8(image).into_rgb8());
    let input = engine
        .prepare_input(ImageSource::from_bytes(image.as_raw(), image.dimensions())?)
        .context("failed to prepare OCR input")?;
    let word_rects = engine
        .detect_words(&input)
        .context("text detection failed")?;
    let line_rects = engine.find_text_lines(&input, &word_rects);
    let lines = engine
        .recognize_text(&input, &line_rects)
        .context("text recognition failed")?;

    Ok(lines
        .into_iter()
        .flatten()
        .map(|line| line.to_string())
        .map(|line| line.trim().to_owned())
        .filter(|line| line.len() > 1)
        .collect::<Vec<_>>()
        .join("\n"))
}

fn load_engine() -> Result<OcrEngine> {
    let detection_model_path = bundled_model_path(DETECTION_MODEL_FILE);
    let recognition_model_path = bundled_model_path(RECOGNITION_MODEL_FILE);

    ensure_file_exists(&detection_model_path, "detection model")?;
    ensure_file_exists(&recognition_model_path, "recognition model")?;

    let detection_model = Model::load_file(&detection_model_path).with_context(|| {
        format!(
            "failed to load detection model from {}",
            detection_model_path.display()
        )
    })?;
    let recognition_model = Model::load_file(&recognition_model_path).with_context(|| {
        format!(
            "failed to load recognition model from {}",
            recognition_model_path.display()
        )
    })?;

    OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection_model),
        recognition_model: Some(recognition_model),
        ..Default::default()
    })
    .context("failed to initialize OCR engine")
}

fn bundled_model_path(file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("models")
        .join(file_name)
}

fn ensure_file_exists(path: &Path, label: &str) -> Result<()> {
    if path.is_file() {
        Ok(())
    } else {
        bail!(
            "{label} not found at {}. Put the OCRS model file there before running OCR.",
            path.display()
        )
    }
}

fn average_luminance(image: &RgbImage) -> f32 {
    let pixel_count = (u64::from(image.width()) * u64::from(image.height())).max(1) as f32;
    let total = image
        .pixels()
        .map(|pixel| {
            let [red, green, blue] = pixel.0;
            0.2126 * f32::from(red) + 0.7152 * f32::from(green) + 0.0722 * f32::from(blue)
        })
        .sum::<f32>();

    total / pixel_count
}

fn preprocess_image(mut image: RgbImage) -> RgbImage {
    if average_luminance(&image) < DARK_IMAGE_LUMINANCE_THRESHOLD {
        for pixel in image.pixels_mut() {
            let [red, green, blue] = pixel.0;
            pixel.0 = [255 - red, 255 - green, 255 - blue];
        }
    }

    image
}
