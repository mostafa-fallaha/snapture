pub mod portal;

use crate::error::{AppResult, SnaptureError};

pub use portal::CapturedImage;

pub fn capture_before_ui() -> AppResult<CapturedImage> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(SnaptureError::from)?;

    runtime.block_on(portal::capture_screenshot())
}
