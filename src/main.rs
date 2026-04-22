mod app;
mod capture;
mod config;
mod editor;
mod error;
mod launch;
mod model;
mod services;
mod tools;
mod ui;

use std::error::Error;

use crate::{
    app::SnaptureApp,
    config::AppConfig,
    launch::{LaunchAction, launch_before_ui},
};

fn main() -> Result<(), Box<dyn Error>> {
    let initial_image = match launch_before_ui() {
        Ok(LaunchAction::Exit) => return Ok(()),
        Ok(LaunchAction::OpenEditor(image)) => image,
        Err(error) => {
            eprintln!("snapture: {error}");
            return Ok(());
        }
    };
    let config = AppConfig::default();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size(config.window_size)
            .with_min_inner_size([960.0, 640.0])
            .with_title(config.app_name),
        ..Default::default()
    };

    let mut initial_image = Some(initial_image);

    eframe::run_native(
        config.app_name,
        options,
        Box::new(move |cc| {
            Ok(Box::new(SnaptureApp::new(
                cc,
                config.clone(),
                initial_image
                    .take()
                    .expect("initial launch image should be consumed once"),
            )))
        }),
    )?;

    Ok(())
}
