pub mod portal;

use std::{sync::mpsc::Sender, time::Duration};

use eframe::egui::{Context, ViewportCommand};

use crate::error::{AppResult, SnaptureError};

pub use portal::CapturedImage;

pub enum CaptureMessage {
    Finished(AppResult<CapturedImage>),
}

pub fn capture_before_ui() -> AppResult<CapturedImage> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(SnaptureError::from)?;

    runtime.block_on(portal::capture_screenshot())
}

pub fn spawn_portal_capture(sender: Sender<CaptureMessage>, ctx: Context, hide_delay: Duration) {
    std::thread::spawn(move || {
        std::thread::sleep(hide_delay);
        let result = capture_before_ui();

        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(ViewportCommand::Focus);
        ctx.request_repaint();

        let _ = sender.send(CaptureMessage::Finished(result));
    });
}
