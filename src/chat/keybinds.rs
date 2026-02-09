use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

const LEADER_TIMEOUT: Duration = Duration::from_millis(1500);

pub enum KeyAction {
    /// No action (key consumed or unrecognized)
    None,
    /// Quit the application
    Quit,
    /// New session
    NewSession,
    /// Show session list
    SessionList,
    /// Toggle sidebar
    ToggleSidebar,
    /// Show model picker
    ModelPicker,
    /// Show drone list (expand sidebar)
    DroneList,
    /// Scroll up (half page)
    ScrollUp,
    /// Scroll down (half page)
    ScrollDown,
    /// Page up
    PageUp,
    /// Page down
    PageDown,
    /// Interrupt current Claude response
    Interrupt,
    /// Pass key to input editor
    PassToInput(KeyEvent),
}

pub struct KeyHandler {
    leader_active: bool,
    leader_time: Option<Instant>,
}

impl Default for KeyHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyHandler {
    pub fn new() -> Self {
        Self {
            leader_active: false,
            leader_time: None,
        }
    }

    /// Process a key event and return the action to take.
    /// Call this from app.handle_key() instead of direct key matching.
    pub fn handle_key(&mut self, key: KeyEvent) -> KeyAction {
        // Check leader timeout
        if self.leader_active {
            if let Some(time) = self.leader_time {
                if time.elapsed() > LEADER_TIMEOUT {
                    self.leader_active = false;
                    self.leader_time = None;
                }
            }
        }

        // Ctrl+C always quits or interrupts
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            if self.leader_active {
                self.leader_active = false;
                return KeyAction::Quit;
            }
            // If Claude is streaming, interrupt instead of quit
            return KeyAction::Interrupt;
        }

        // Leader key: Ctrl+X
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('x') {
            self.leader_active = true;
            self.leader_time = Some(Instant::now());
            return KeyAction::None;
        }

        // Leader key sequences
        if self.leader_active {
            self.leader_active = false;
            self.leader_time = None;

            return match key.code {
                KeyCode::Char('n') => KeyAction::NewSession,
                KeyCode::Char('l') => KeyAction::SessionList,
                KeyCode::Char('b') => KeyAction::ToggleSidebar,
                KeyCode::Char('m') => KeyAction::ModelPicker,
                KeyCode::Char('d') => KeyAction::DroneList,
                _ => KeyAction::None, // Unknown leader sequence
            };
        }

        // Global keys (not in leader mode)
        match (key.modifiers, key.code) {
            // Escape: close dialog or quit
            (_, KeyCode::Esc) => KeyAction::Quit,

            // Scroll keys
            (_, KeyCode::PageUp) => KeyAction::PageUp,
            (_, KeyCode::PageDown) => KeyAction::PageDown,
            (m, KeyCode::Char('u')) if m.contains(KeyModifiers::CONTROL) => KeyAction::ScrollUp,
            (m, KeyCode::Char('d')) if m.contains(KeyModifiers::CONTROL) => KeyAction::ScrollDown,

            // Everything else goes to input editor
            _ => KeyAction::PassToInput(key),
        }
    }

    /// Check if leader key is currently active (for rendering indicator)
    pub fn is_leader_active(&self) -> bool {
        self.leader_active
    }
}
