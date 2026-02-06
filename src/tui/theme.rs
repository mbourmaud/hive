use ratatui::style::Color;

/// Theme variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeVariant {
    Dark,
    Light,
}

/// Theme configuration for the TUI
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Theme {
    /// Theme variant (dark or light)
    pub variant: ThemeVariant,

    // Background colors
    pub bg_primary: Color,
    pub bg_secondary: Color,
    pub bg_sidebar: Color,
    pub bg_input: Color,
    pub bg_popup: Color,

    // Foreground/text colors
    pub fg_primary: Color,
    pub fg_secondary: Color,
    pub fg_muted: Color,
    pub fg_bright: Color,

    // Border colors
    pub border_normal: Color,
    pub border_focused: Color,
    pub border_sidebar: Color,
    pub border_input: Color,
    pub border_popup: Color,

    // Accent colors
    pub accent_primary: Color,
    pub accent_secondary: Color,
    pub accent_success: Color,
    pub accent_warning: Color,
    pub accent_error: Color,
    pub accent_info: Color,

    // Selection/highlight colors
    pub selection_bg: Color,
    pub selection_fg: Color,

    // Message colors
    pub msg_user: Color,
    pub msg_assistant: Color,
    pub msg_system: Color,

    // Footer colors
    pub footer_bg: Color,
    pub footer_fg: Color,
    pub footer_key_bg: Color,
    pub footer_key_fg: Color,
}

impl Theme {
    /// Create a new dark theme (default)
    pub fn dark() -> Self {
        Self {
            variant: ThemeVariant::Dark,

            // Backgrounds - dark palette
            bg_primary: Color::Black,
            bg_secondary: Color::Rgb(20, 20, 30),
            bg_sidebar: Color::Rgb(15, 15, 25),
            bg_input: Color::Black,
            bg_popup: Color::Black,

            // Foreground - light text on dark
            fg_primary: Color::Rgb(220, 220, 220),
            fg_secondary: Color::Rgb(180, 180, 180),
            fg_muted: Color::Rgb(100, 100, 110),
            fg_bright: Color::White,

            // Borders - subtle cyan/blue tones
            border_normal: Color::Rgb(60, 60, 80),
            border_focused: Color::Cyan,
            border_sidebar: Color::Rgb(70, 130, 180),
            border_input: Color::Magenta,
            border_popup: Color::Yellow,

            // Accents - vibrant colors
            accent_primary: Color::Cyan,
            accent_secondary: Color::Magenta,
            accent_success: Color::Green,
            accent_warning: Color::Yellow,
            accent_error: Color::Red,
            accent_info: Color::Blue,

            // Selection
            selection_bg: Color::Rgb(80, 80, 120),
            selection_fg: Color::Yellow,

            // Messages
            msg_user: Color::Cyan,
            msg_assistant: Color::Green,
            msg_system: Color::Yellow,

            // Footer
            footer_bg: Color::Rgb(40, 40, 50),
            footer_fg: Color::Rgb(200, 200, 200),
            footer_key_bg: Color::Gray,
            footer_key_fg: Color::Black,
        }
    }

    /// Create a new light theme
    pub fn light() -> Self {
        Self {
            variant: ThemeVariant::Light,

            // Backgrounds - light palette
            bg_primary: Color::Rgb(250, 250, 250),
            bg_secondary: Color::Rgb(240, 240, 245),
            bg_sidebar: Color::Rgb(245, 245, 250),
            bg_input: Color::White,
            bg_popup: Color::White,

            // Foreground - dark text on light
            fg_primary: Color::Rgb(40, 40, 40),
            fg_secondary: Color::Rgb(80, 80, 80),
            fg_muted: Color::Rgb(150, 150, 160),
            fg_bright: Color::Black,

            // Borders - darker tones for contrast
            border_normal: Color::Rgb(200, 200, 210),
            border_focused: Color::Rgb(0, 150, 200),
            border_sidebar: Color::Rgb(100, 150, 200),
            border_input: Color::Rgb(180, 0, 180),
            border_popup: Color::Rgb(200, 150, 0),

            // Accents - slightly muted for light background
            accent_primary: Color::Rgb(0, 140, 180),
            accent_secondary: Color::Rgb(170, 0, 170),
            accent_success: Color::Rgb(0, 150, 0),
            accent_warning: Color::Rgb(200, 140, 0),
            accent_error: Color::Rgb(200, 0, 0),
            accent_info: Color::Rgb(0, 100, 200),

            // Selection
            selection_bg: Color::Rgb(220, 220, 240),
            selection_fg: Color::Rgb(180, 100, 0),

            // Messages
            msg_user: Color::Rgb(0, 120, 160),
            msg_assistant: Color::Rgb(0, 130, 0),
            msg_system: Color::Rgb(180, 120, 0),

            // Footer
            footer_bg: Color::Rgb(220, 220, 230),
            footer_fg: Color::Rgb(50, 50, 50),
            footer_key_bg: Color::Rgb(180, 180, 190),
            footer_key_fg: Color::White,
        }
    }

    /// Toggle between dark and light themes
    pub fn toggle(&self) -> Self {
        match self.variant {
            ThemeVariant::Dark => Self::light(),
            ThemeVariant::Light => Self::dark(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_toggle() {
        let dark = Theme::dark();
        assert_eq!(dark.variant, ThemeVariant::Dark);

        let light = dark.toggle();
        assert_eq!(light.variant, ThemeVariant::Light);

        let dark_again = light.toggle();
        assert_eq!(dark_again.variant, ThemeVariant::Dark);
    }

    #[test]
    fn test_default_theme() {
        let theme = Theme::default();
        assert_eq!(theme.variant, ThemeVariant::Dark);
    }
}
