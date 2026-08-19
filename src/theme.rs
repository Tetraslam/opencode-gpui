pub mod color {
    pub const BASE: u32 = 0x000f_0f14;
    pub const SURFACE: u32 = 0x0016_161e;
    pub const ELEVATED: u32 = 0x001e_1e28;
    pub const HOVER: u32 = 0x0024_2430;
    pub const SELECTED: u32 = 0x0029_2937;
    pub const BORDER_SUBTLE: u32 = 0x001e_1e2a;
    pub const BORDER: u32 = 0x0028_283a;
    pub const TEXT: u32 = 0x00d5_d5e0;
    pub const TEXT_BRIGHT: u32 = 0x00eb_ebf5;
    pub const TEXT_DIM: u32 = 0x0088_88a0;
    pub const TEXT_MUTED: u32 = 0x0050_506a;
    pub const ACCENT: u32 = 0x00b4_a0e0;
    pub const BLUE: u32 = 0x0088_b0f0;
    pub const CYAN: u32 = 0x0070_c8d8;
    pub const GREEN: u32 = 0x00a0_d070;
    pub const YELLOW: u32 = 0x00e0_c070;
    pub const RED: u32 = 0x00e0_7070;
    pub const TOOL: u32 = YELLOW;
    pub const REASONING: u32 = ACCENT;
}

pub mod size {
    pub const TITLEBAR: f32 = 32.0;
    pub const ACTIVITY_RAIL: f32 = 42.0;
    pub const PANE_HEADER: f32 = 26.0;
    pub const SESSION_PANE: f32 = 286.0;
    pub const SESSION_ROW: f32 = 34.0;
    pub const INSPECTOR: f32 = 330.0;
    pub const STATUSLINE: f32 = 24.0;
    pub const MESSAGE_HEADER: f32 = 24.0;
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
