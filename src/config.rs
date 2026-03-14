use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::model::types::RgbaColor;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub app_name: &'static str,
    pub window_size: [f32; 2],
    pub history_limit: usize,
    pub capture_hide_delay_ms: u64,
    pub min_zoom: f32,
    pub max_zoom: f32,
    pub default_color: RgbaColor,
    pub default_stroke_thickness: f32,
    pub default_text_size: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app_name: "snapture",
            window_size: [1440.0, 920.0],
            history_limit: 64,
            capture_hide_delay_ms: 250,
            min_zoom: 0.25,
            max_zoom: 4.0,
            default_color: RgbaColor::from_rgba(255, 59, 48, 255),
            default_stroke_thickness: 4.0,
            default_text_size: 28.0,
        }
    }
}

impl AppConfig {
    pub fn default_save_path(&self) -> PathBuf {
        let base_dir = dirs::picture_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."));
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);

        base_dir.join(format!("snapture-{timestamp}.png"))
    }
}
