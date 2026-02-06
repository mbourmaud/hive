use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeMode {
    Dark,
    Light,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub mode: ThemeMode,

    // Base colors
    pub bg: Color,
    pub fg: Color,
    pub fg_dim: Color,

    // Accent colors
    pub accent: Color,
    pub accent_secondary: Color,

    // Status colors
    pub success: Color,
    pub error: Color,
    pub warning: Color,
    pub info: Color,

    // UI elements
    pub border: Color,
    pub border_focused: Color,
    pub border_unfocused: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,

    // Chat
    pub user_color: Color,
    pub assistant_color: Color,
    pub system_color: Color,
    pub code_color: Color,

    // Sidebar
    pub drone_running: Color,
    pub drone_completed: Color,
    pub drone_error: Color,
    pub drone_stopped: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            bg: Color::Reset,
            fg: Color::White,
            fg_dim: Color::DarkGray,
            accent: Color::Yellow,
            accent_secondary: Color::Cyan,
            success: Color::Green,
            error: Color::Red,
            warning: Color::Yellow,
            info: Color::Cyan,
            border: Color::DarkGray,
            border_focused: Color::Yellow,
            border_unfocused: Color::DarkGray,
            selection_bg: Color::Yellow,
            selection_fg: Color::Black,
            user_color: Color::Cyan,
            assistant_color: Color::Green,
            system_color: Color::DarkGray,
            code_color: Color::Yellow,
            drone_running: Color::Green,
            drone_completed: Color::Green,
            drone_error: Color::Red,
            drone_stopped: Color::DarkGray,
        }
    }

    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            bg: Color::Reset,
            fg: Color::Black,
            fg_dim: Color::DarkGray,
            accent: Color::Blue,
            accent_secondary: Color::Magenta,
            success: Color::Green,
            error: Color::Red,
            warning: Color::Yellow,
            info: Color::Blue,
            border: Color::DarkGray,
            border_focused: Color::Blue,
            border_unfocused: Color::DarkGray,
            selection_bg: Color::Blue,
            selection_fg: Color::White,
            user_color: Color::Blue,
            assistant_color: Color::Magenta,
            system_color: Color::DarkGray,
            code_color: Color::Red,
            drone_running: Color::Green,
            drone_completed: Color::Green,
            drone_error: Color::Red,
            drone_stopped: Color::DarkGray,
        }
    }

    pub fn toggle(&self) -> Self {
        match self.mode {
            ThemeMode::Dark => Self::light(),
            ThemeMode::Light => Self::dark(),
        }
    }

    // Convenience style builders

    pub fn border_style(&self, focused: bool) -> Style {
        if focused {
            Style::default().fg(self.border_focused)
        } else {
            Style::default().fg(self.border_unfocused)
        }
    }

    pub fn highlight_style(&self) -> Style {
        Style::default()
            .fg(self.selection_fg)
            .bg(self.selection_bg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn accent_style(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn dim_style(&self) -> Style {
        Style::default().fg(self.fg_dim)
    }

    pub fn hint_key_style(&self) -> Style {
        Style::default().fg(self.selection_fg).bg(self.fg_dim)
    }

    pub fn hint_desc_style(&self) -> Style {
        Style::default().fg(self.fg_dim)
    }
}

pub const HIVE_LOGO: &str = r"
 \  /
  \/
  /\  HIVE
 /  \
";

pub const HIVE_LOGO_COMPACT: &str = "\u{2b21} HIVE";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dark_theme() {
        let theme = Theme::dark();
        assert_eq!(theme.mode, ThemeMode::Dark);
        assert_eq!(theme.accent, Color::Yellow);
    }

    #[test]
    fn test_light_theme() {
        let theme = Theme::light();
        assert_eq!(theme.mode, ThemeMode::Light);
        assert_eq!(theme.accent, Color::Blue);
    }

    #[test]
    fn test_toggle() {
        let dark = Theme::dark();
        let light = dark.toggle();
        assert_eq!(light.mode, ThemeMode::Light);
        let back = light.toggle();
        assert_eq!(back.mode, ThemeMode::Dark);
    }

    #[test]
    fn test_border_style() {
        let theme = Theme::dark();
        let focused = theme.border_style(true);
        let unfocused = theme.border_style(false);
        assert_ne!(focused, unfocused);
    }
}
