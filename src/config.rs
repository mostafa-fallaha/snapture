use crate::model::types::RgbaColor;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub app_name: &'static str,
    pub window_size: [f32; 2],
    pub history_limit: usize,
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
            min_zoom: 0.25,
            max_zoom: 4.0,
            default_color: RgbaColor::from_rgba(255, 59, 48, 255),
            default_stroke_thickness: 4.0,
            default_text_size: 28.0,
        }
    }
}
