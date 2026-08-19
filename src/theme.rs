pub mod color {
    pub const BASE: u32 = 0x0011_1117;
    pub const SURFACE: u32 = 0x001e_1d27;
    pub const ELEVATED: u32 = 0x0026_2431;
    pub const HOVER: u32 = 0x002d_2a39;
    pub const SELECTED: u32 = 0x0034_2f42;
    pub const BORDER_SUBTLE: u32 = 0x0023_212c;
    pub const BORDER: u32 = 0x0030_2d3a;
    pub const TEXT: u32 = 0x00b8_b6c1;
    pub const TEXT_BRIGHT: u32 = 0x00d2_ced8;
    pub const TEXT_DIM: u32 = 0x0085_818f;
    pub const TEXT_MUTED: u32 = 0x005f_5b69;
    pub const ACCENT: u32 = 0x00a9_9ac6;
    pub const BLUE: u32 = 0x007f_9fbd;
    pub const CYAN: u32 = 0x0078_a8aa;
    pub const GREEN: u32 = 0x008f_a879;
    pub const YELLOW: u32 = 0x00b8_a06a;
    pub const RED: u32 = 0x00c4_7878;
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
}
