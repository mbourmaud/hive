use super::chat::ChatPanel;
use super::claude::ClaudeProcess;
use super::commands::CommandPalette;
use super::dialogs::ModalDialog;
use super::file_picker::FilePicker;
use super::input::InputWidget;
use super::messages::ChatMessage;
use super::monitor;
use super::sessions::SessionManager;
use super::sidebar::SidebarState;
use super::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sidebar,
    Chat,
    Input,
}

pub struct App<'a> {
    pub running: bool,
    pub focus: Focus,
    pub sidebar_visible: bool,
    pub theme: Theme,
    pub dark_mode: bool,
    pub chat: ChatPanel,
    pub input: InputWidget<'a>,
    pub sidebar: SidebarState,
    pub sessions: SessionManager,
    pub commands: CommandPalette,
    pub file_picker: FilePicker,
    pub claude: Option<ClaudeProcess>,
    pub permission_dialog: Option<ModalDialog>,
    pub status_message: Option<(String, std::time::Instant)>,
}

impl<'a> Default for App<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> App<'a> {
    pub fn new() -> Self {
        let mut sidebar = SidebarState::new();
        sidebar.refresh();
        Self {
            running: true,
            focus: Focus::Input,
            sidebar_visible: true,
            theme: Theme::dark(),
            dark_mode: true,
            chat: ChatPanel::new(),
            input: InputWidget::new(),
            sidebar,
            sessions: SessionManager::new(),
            commands: CommandPalette::new(),
            file_picker: FilePicker::new(),
            claude: None,
            permission_dialog: None,
            status_message: None,
        }
    }

    pub fn refresh_sidebar(&mut self) {
        self.sidebar.refresh();
    }

    pub fn toggle_sidebar(&mut self) {
        self.sidebar_visible = !self.sidebar_visible;
        if !self.sidebar_visible && self.focus == Focus::Sidebar {
            self.focus = Focus::Input;
        }
    }

    pub fn toggle_theme(&mut self) {
        self.dark_mode = !self.dark_mode;
        self.theme = if self.dark_mode {
            Theme::dark()
        } else {
            Theme::light()
        };
    }

    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Sidebar => Focus::Chat,
            Focus::Chat => Focus::Input,
            Focus::Input => {
                if self.sidebar_visible {
                    Focus::Sidebar
                } else {
                    Focus::Chat
                }
            }
        };
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), std::time::Instant::now()));
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn on_submit(&mut self, text: String) {
        // Handle slash commands
        if text.starts_with('/') {
            self.handle_slash_command(&text);
            return;
        }

        // Add user message to chat
        self.chat.add_message(ChatMessage::user(&text));

        // Send to Claude if connected
        if let Some(claude) = &mut self.claude {
            if let Err(e) = claude.send_message(&text) {
                self.chat
                    .add_message(ChatMessage::error(format!("Failed to send: {}", e)));
            }
        } else {
            self.chat.add_message(ChatMessage::system(
                "Not connected to Claude. Messages are local only.",
            ));
        }
    }

    fn handle_slash_command(&mut self, cmd: &str) {
        let cmd = cmd.trim();
        match cmd {
            "/clear" => {
                self.chat = ChatPanel::new();
                self.set_status("Chat cleared");
            }
            "/theme" => {
                self.toggle_theme();
                let mode = if self.dark_mode { "dark" } else { "light" };
                self.set_status(format!("Switched to {} theme", mode));
            }
            "/quit" => {
                self.quit();
            }
            "/refresh" => {
                self.refresh_sidebar();
                self.set_status("Drones refreshed");
            }
            "/stop" => {
                if let Some(drone) = self.sidebar.selected_drone() {
                    let name = drone.name.clone();
                    match super::drone_actions::stop_drone(&name) {
                        Ok(msg) => self.set_status(format!("Stopped {}: {}", name, msg.trim())),
                        Err(e) => self.set_status(format!("Failed to stop {}: {}", name, e)),
                    }
                    self.refresh_sidebar();
                } else {
                    self.set_status("No drone selected");
                }
            }
            "/clean" => {
                if let Some(drone) = self.sidebar.selected_drone() {
                    let name = drone.name.clone();
                    match super::drone_actions::clean_drone(&name) {
                        Ok(msg) => self.set_status(format!("Cleaned {}: {}", name, msg.trim())),
                        Err(e) => self.set_status(format!("Failed to clean {}: {}", name, e)),
                    }
                    self.refresh_sidebar();
                } else {
                    self.set_status("No drone selected");
                }
            }
            "/logs" => {
                if let Some(drone) = self.sidebar.selected_drone() {
                    let name = drone.name.clone();
                    match super::drone_actions::view_logs(&name) {
                        Ok(logs) => {
                            self.chat.add_message(ChatMessage::system(format!(
                                "Logs for {}:\n{}",
                                name, logs
                            )));
                        }
                        Err(e) => self.set_status(format!("Failed to get logs: {}", e)),
                    }
                } else {
                    self.set_status("No drone selected");
                }
            }
            _ => {
                self.set_status(format!("Unknown command: {}", cmd));
            }
        }
    }

    /// Selected drone from sidebar
    pub fn selected_drone_name(&self) -> Option<String> {
        self.sidebar.selected_drone().map(|d| d.name.clone())
    }
}

impl SidebarState {
    pub fn selected_drone(&self) -> Option<&monitor::DroneInfo> {
        self.drones.get(self.selected_index)
    }
}
