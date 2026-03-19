use eframe::egui::{self, Color32, CornerRadius, Frame, Margin, Shadow, Stroke, vec2};

pub const APP_BG: Color32 = Color32::from_rgb(16, 19, 24);
pub const PANEL_BG: Color32 = Color32::from_rgb(22, 26, 33);
pub const SECTION_BG: Color32 = Color32::from_rgb(28, 33, 41);
pub const SECTION_BG_HOVER: Color32 = Color32::from_rgb(34, 40, 50);
pub const INPUT_BG: Color32 = Color32::from_rgb(18, 22, 28);
pub const BORDER: Color32 = Color32::from_rgb(58, 66, 80);
pub const BORDER_STRONG: Color32 = Color32::from_rgb(78, 90, 108);
pub const TEXT: Color32 = Color32::from_rgb(235, 239, 244);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(150, 159, 171);
pub const ACCENT: Color32 = Color32::from_rgb(76, 145, 220);
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(95, 163, 238);
pub const ACCENT_ACTIVE: Color32 = Color32::from_rgb(58, 124, 196);
pub const SUCCESS: Color32 = Color32::from_rgb(84, 182, 130);
pub const DANGER: Color32 = Color32::from_rgb(204, 92, 92);

pub const CONTROL_HEIGHT: f32 = 30.0;

pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = vec2(8.0, 8.0);
    style.spacing.button_padding = vec2(10.0, 6.0);
    style.spacing.interact_size = vec2(40.0, CONTROL_HEIGHT);
    style.spacing.slider_width = 122.0;
    style.spacing.text_edit_width = 220.0;
    style.spacing.window_margin = Margin::same(12);

    let visuals = &mut style.visuals;
    *visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(TEXT);
    visuals.weak_text_color = Some(TEXT_MUTED);
    visuals.panel_fill = APP_BG;
    visuals.faint_bg_color = PANEL_BG;
    visuals.extreme_bg_color = INPUT_BG;
    visuals.text_edit_bg_color = Some(INPUT_BG);
    visuals.code_bg_color = INPUT_BG;
    visuals.window_corner_radius = CornerRadius::same(12);
    visuals.window_shadow = Shadow {
        offset: [0, 10],
        blur: 28,
        spread: 0,
        color: Color32::from_black_alpha(110),
    };
    visuals.window_fill = PANEL_BG;
    visuals.window_stroke = Stroke::new(1.0, BORDER);
    visuals.menu_corner_radius = CornerRadius::same(10);
    visuals.popup_shadow = Shadow {
        offset: [0, 8],
        blur: 22,
        spread: 0,
        color: Color32::from_black_alpha(96),
    };
    visuals.selection.bg_fill =
        Color32::from_rgba_unmultiplied(ACCENT.r(), ACCENT.g(), ACCENT.b(), 52);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.hyperlink_color = ACCENT_HOVER;
    visuals.warn_fg_color = Color32::from_rgb(232, 168, 84);
    visuals.error_fg_color = DANGER;
    visuals.button_frame = true;
    visuals.slider_trailing_fill = true;
    visuals.handle_shape = egui::style::HandleShape::Rect { aspect_ratio: 0.65 };

    visuals.widgets.noninteractive.bg_fill = PANEL_BG;
    visuals.widgets.noninteractive.weak_bg_fill = PANEL_BG;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(10);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);

    visuals.widgets.inactive.bg_fill = SECTION_BG;
    visuals.widgets.inactive.weak_bg_fill = SECTION_BG;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(8);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);

    visuals.widgets.hovered.bg_fill = SECTION_BG_HOVER;
    visuals.widgets.hovered.weak_bg_fill = SECTION_BG_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, BORDER_STRONG);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(8);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT);

    visuals.widgets.active.bg_fill = ACCENT_ACTIVE;
    visuals.widgets.active.weak_bg_fill = ACCENT_ACTIVE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT_HOVER);
    visuals.widgets.active.corner_radius = CornerRadius::same(8);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, TEXT);

    visuals.widgets.open = visuals.widgets.active;

    ctx.set_style(style);
}

pub fn topbar_frame() -> Frame {
    Frame::new()
        .fill(PANEL_BG)
        .stroke(Stroke::new(1.0, BORDER))
        .inner_margin(Margin::symmetric(12, 10))
}

pub fn sidebar_frame() -> Frame {
    Frame::new()
        .fill(PANEL_BG)
        .stroke(Stroke::new(1.0, BORDER))
        .inner_margin(Margin::same(12))
}

pub fn central_frame() -> Frame {
    Frame::new().fill(APP_BG).inner_margin(Margin::same(12))
}

pub fn section_frame() -> Frame {
    Frame::new()
        .fill(SECTION_BG)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(10))
}

pub fn floating_frame() -> Frame {
    Frame::new()
        .fill(PANEL_BG)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(12))
        .shadow(Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: Color32::from_black_alpha(92),
        })
}
