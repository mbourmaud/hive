use ratatui::style::{Color, Modifier, Style};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub border: Color,
    pub border_focused: Color,
    pub accent: Color,
    pub error: Color,
    pub success: Color,
    pub warning: Color,
    pub muted: Color,
    pub user_msg: Color,
    pub assistant_msg: Color,
    pub system_msg: Color,
    pub code_bg: Color,
    pub code_fg: Color,
    pub selection: Color,
}

#[allow(dead_code)]
impl Theme {
    pub fn dark() -> Self {
        Self {
            bg: Color::Rgb(30, 30, 46),
            fg: Color::Rgb(205, 214, 244),
            border: Color::Rgb(88, 91, 112),
            border_focused: Color::Rgb(137, 180, 250),
            accent: Color::Rgb(137, 180, 250),
            error: Color::Rgb(243, 139, 168),
            success: Color::Rgb(166, 227, 161),
            warning: Color::Rgb(249, 226, 175),
            muted: Color::Rgb(108, 112, 134),
            user_msg: Color::Rgb(166, 227, 161),
            assistant_msg: Color::Rgb(137, 180, 250),
            system_msg: Color::Rgb(249, 226, 175),
            code_bg: Color::Rgb(49, 50, 68),
            code_fg: Color::Rgb(166, 227, 161),
            selection: Color::Rgb(69, 71, 90),
        }
    }

    pub fn light() -> Self {
        Self {
            bg: Color::Rgb(239, 241, 245),
            fg: Color::Rgb(76, 79, 105),
            border: Color::Rgb(172, 176, 190),
            border_focused: Color::Rgb(30, 102, 245),
            accent: Color::Rgb(30, 102, 245),
            error: Color::Rgb(210, 15, 57),
            success: Color::Rgb(64, 160, 43),
            warning: Color::Rgb(223, 142, 29),
            muted: Color::Rgb(140, 143, 161),
            user_msg: Color::Rgb(64, 160, 43),
            assistant_msg: Color::Rgb(30, 102, 245),
            system_msg: Color::Rgb(223, 142, 29),
            code_bg: Color::Rgb(220, 224, 232),
            code_fg: Color::Rgb(64, 160, 43),
            selection: Color::Rgb(204, 208, 218),
        }
    }

    pub fn base_style(&self) -> Style {
        Style::default().fg(self.fg).bg(self.bg)
    }

    pub fn border_style(&self, focused: bool) -> Style {
        if focused {
            Style::default().fg(self.border_focused)
        } else {
            Style::default().fg(self.border)
        }
    }

    pub fn accent_style(&self) -> Style {
        Style::default().fg(self.accent)
    }

    pub fn bold_accent_style(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn error_style(&self) -> Style {
        Style::default().fg(self.error)
    }

    pub fn success_style(&self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn warning_style(&self) -> Style {
        Style::default().fg(self.warning)
    }

    pub fn muted_style(&self) -> Style {
        Style::default().fg(self.muted)
    }
}
