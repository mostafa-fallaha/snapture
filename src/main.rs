mod app;
mod capture;
mod config;
mod editor;
mod error;
mod model;
mod services;
mod tools;
mod ui;

use std::error::Error;

use crate::{app::SnaptureApp, capture::capture_before_ui, config::AppConfig};

fn main() -> Result<(), Box<dyn Error>> {
    let config = AppConfig::default();
    let initial_capture = match capture_before_ui() {
        Ok(capture) => capture,
        Err(error) => {
            eprintln!("snapture: screenshot capture failed: {error}");
            return Ok(());
        }
    };

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size(config.window_size)
            .with_min_inner_size([960.0, 640.0])
            .with_title(config.app_name),
        ..Default::default()
    };

    let mut initial_capture = Some(initial_capture);

    eframe::run_native(
        config.app_name,
        options,
        Box::new(move |cc| {
            Ok(Box::new(SnaptureApp::new(
                cc,
                config.clone(),
                initial_capture
                    .take()
                    .expect("initial capture should be consumed once"),
            )))
        }),
    )?;

    Ok(())
}
