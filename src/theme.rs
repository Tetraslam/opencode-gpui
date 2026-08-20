pub mod color {
    pub const BASE: u32 = 0x001a_1b26;
    pub const SURFACE: u32 = 0x0028_2a3b;
    pub const ELEVATED: u32 = 0x002f_3145;
    pub const HOVER: u32 = 0x0038_3b53;
    pub const SELECTED: u32 = 0x003d_405a;
    pub const BORDER_SUBTLE: u32 = 0x0044_4764;
    pub const BORDER: u32 = 0x004b_4e6e;
    pub const TEXT: u32 = 0x00a9_b1d6;
    pub const TEXT_BRIGHT: u32 = 0x00c0_c7e8;
    pub const TEXT_DIM: u32 = 0x0092_99bd;
    pub const TEXT_MUTED: u32 = 0x008f_96b8;
    pub const ACCENT: u32 = 0x0044_9dab;
    pub const BLUE: u32 = 0x007a_a2f7;
    pub const CYAN: u32 = 0x0044_9dab;
    pub const GREEN: u32 = 0x009e_ce6a;
    pub const YELLOW: u32 = 0x00e0_af68;
    pub const RED: u32 = 0x00f7_768e;
    pub const DIFF_ADDED_BG: u32 = 0x0037_4235;
    pub const DIFF_REMOVED_BG: u32 = 0x004b_2f3d;
    pub const DIFF_CONTEXT_BG: u32 = 0x0028_2a3b;
    pub const TOOL: u32 = YELLOW;
    pub const REASONING: u32 = ACCENT;
}

pub mod size {
    pub const EDGE_INSET: f32 = 12.0;
    pub const GAP: f32 = 8.0;
    pub const MARKER_COL: f32 = 10.0;
    pub const TOOL_CONTENT_X: f32 = 56.0;
    pub const INDENT_STEP: f32 = 16.0;
    pub const AGE_COL: f32 = 40.0;
    pub const TITLEBAR: f32 = 32.0;
    pub const ACTIVITY_RAIL: f32 = 42.0;
    pub const PANE_HEADER: f32 = 26.0;
    pub const SESSION_PANE: f32 = 286.0;
    pub const SESSION_ROW: f32 = 34.0;
    pub const INSPECTOR: f32 = 420.0;
    pub const STATUSLINE: f32 = 24.0;
    pub const MESSAGE_HEADER: f32 = 26.0;
}

pub const UI_FONT: &str = "Noto Sans";
pub const MONO_FONT: &str = "FiraCode Nerd Font";

#[cfg(test)]
mod tests {
    use super::color;

    #[test]
    fn palette_roles_are_distinct() {
        assert_ne!(color::BASE, color::SURFACE);
        assert_ne!(color::SURFACE, color::ELEVATED);
        assert_ne!(color::TEXT, color::TEXT_DIM);
        assert_ne!(color::GREEN, color::RED);
    }

    #[test]
    fn reading_text_meets_dark_surface_contrast_floor() {
        assert!(contrast(color::TEXT, color::BASE) >= 4.5);
        assert!(contrast(color::TEXT, color::SURFACE) >= 4.5);
        assert!(contrast(color::TEXT_DIM, color::BASE) >= 4.5);
        assert!(contrast(color::TEXT_MUTED, color::DIFF_CONTEXT_BG) >= 4.5);
        assert!(contrast(color::TEXT, color::DIFF_ADDED_BG) >= 4.5);
        assert!(contrast(color::TEXT, color::DIFF_REMOVED_BG) >= 4.5);
    }

    fn contrast(foreground: u32, background: u32) -> f64 {
        let foreground = luminance(foreground);
        let background = luminance(background);
        (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05)
    }

    fn luminance(color: u32) -> f64 {
        let channel = |shift: u32| {
            let value = f64::from((color >> shift) & 0xff_u32) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
    }
}
