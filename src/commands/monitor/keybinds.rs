use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Color;
use ratatui::Terminal;
use std::time::{Duration, Instant};

use crate::types::DroneState;

use super::drone_actions::{
    handle_clean_drone, handle_new_drone, handle_resume_drone, handle_stop_drone,
};
use super::state::TuiState;

pub(crate) enum KeyAction {
    Continue,
    Break,
}

impl TuiState {
    pub fn handle_key<B: ratatui::backend::Backend>(
        &mut self,
        key: KeyEvent,
        terminal: &mut Terminal<B>,
    ) -> Result<KeyAction> {
        // Convert display index to actual drone index
        let current_drone_idx =
            if !self.display_order.is_empty() && self.selected_index < self.display_order.len() {
                self.display_order[self.selected_index]
            } else {
                0
            };

        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                if self.tools_view.is_some() {
                    self.tools_view = None;
                } else if self.messages_view.is_some() {
                    self.messages_view = None;
                    self.messages_scroll = 0;
                    self.messages_selected_index = usize::MAX;
                } else {
                    return Ok(KeyAction::Break);
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.messages_view.is_some() {
                    // Switch from auto-scroll (usize::MAX) to manual mode
                    if self.messages_selected_index == usize::MAX {
                        self.messages_selected_index = 0;
                    } else {
                        self.messages_selected_index += 1;
                    }
                } else if !self.drones.is_empty() && self.selected_index < self.drones.len() - 1 {
                    self.selected_index += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.messages_view.is_some() {
                    // Switch from auto-scroll (usize::MAX) to manual mode
                    if self.messages_selected_index == usize::MAX {
                        self.messages_selected_index = 0;
                    } else {
                        self.messages_selected_index =
                            self.messages_selected_index.saturating_sub(1);
                    }
                } else {
                    self.selected_index = self.selected_index.saturating_sub(1);
                }
            }
            KeyCode::Char('g') | KeyCode::Home => {
                if self.messages_view.is_some() {
                    // Jump to first message
                    self.messages_selected_index = 0;
                }
            }
            KeyCode::Char('G') | KeyCode::End => {
                if self.messages_view.is_some() {
                    // Jump to last message / auto-scroll mode
                    self.messages_selected_index = usize::MAX;
                }
            }
            KeyCode::Enter | KeyCode::Right => {
                if !self.drones.is_empty() {
                    let drone_name = &self.drones[current_drone_idx].0;
                    if self.expanded_drones.contains(drone_name) {
                        self.expanded_drones.remove(drone_name);
                    } else {
                        self.expanded_drones.insert(drone_name.clone());
                    }
                }
            }
            KeyCode::Left => {
                if !self.drones.is_empty() {
                    let drone_name = &self.drones[current_drone_idx].0;
                    self.expanded_drones.remove(drone_name);
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => match handle_new_drone(terminal) {
                Ok(Some(msg)) => {
                    self.set_message(msg, Color::Green);
                }
                Ok(None) => {}
                Err(e) => {
                    self.set_message(format!("Error: {}", e), Color::Red);
                }
            },
            KeyCode::Char('m') | KeyCode::Char('M') => {
                if !self.drones.is_empty() {
                    let drone_name = self.drones[current_drone_idx].0.clone();
                    self.messages_view = Some(drone_name);
                    self.messages_scroll = 0;
                    self.messages_selected_index = usize::MAX; // Start in auto-scroll mode
                }
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                if self.tools_view.is_some() {
                    self.tools_view = None;
                } else if !self.drones.is_empty() {
                    let drone_name = self.drones[current_drone_idx].0.clone();
                    self.tools_view = Some(drone_name);
                }
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                if !self.drones.is_empty() {
                    let drone_name = self.drones[current_drone_idx].0.clone();
                    match handle_stop_drone(&drone_name) {
                        Ok(msg) => self.set_message(msg, Color::Green),
                        Err(e) => self.set_message(format!("Error: {}", e), Color::Red),
                    }
                }
            }
            KeyCode::Char('D') => {
                if !self.drones.is_empty() {
                    let drone_name = self.drones[current_drone_idx].0.clone();
                    if let Some((ref pending, when)) = self.pending_clean {
                        if pending == &drone_name {
                            let elapsed = when.elapsed();
                            if elapsed >= Duration::from_secs(3) {
                                // Held D for 3 seconds — execute clean
                                let name = pending.clone();
                                self.pending_clean = None;
                                match handle_clean_drone(&name) {
                                    Ok(msg) => self.set_message(msg, Color::Green),
                                    Err(e) => self.set_message(format!("Error: {}", e), Color::Red),
                                }
                            } else {
                                // Still holding — update countdown
                                let remaining = 3 - elapsed.as_secs();
                                self.set_message(
                                    format!(
                                        "Cleaning '{}' in {}s... (release to cancel)",
                                        drone_name, remaining
                                    ),
                                    Color::Yellow,
                                );
                            }
                        } else {
                            // Different drone — reset
                            self.pending_clean = Some((drone_name.clone(), Instant::now()));
                            self.set_message(
                                format!("Cleaning '{}' in 3s... (release to cancel)", drone_name),
                                Color::Yellow,
                            );
                        }
                    } else {
                        // First D — start countdown
                        self.pending_clean = Some((drone_name.clone(), Instant::now()));
                        self.set_message(
                            format!("Cleaning '{}' in 3s... (release to cancel)", drone_name),
                            Color::Yellow,
                        );
                    }
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                if !self.drones.is_empty() {
                    let drone_name = self.drones[current_drone_idx].0.clone();
                    let status = &self.drones[current_drone_idx].1;

                    if matches!(status.status, DroneState::Completed | DroneState::Stopped) {
                        match handle_resume_drone(&drone_name) {
                            Ok(msg) => self.set_message(msg, Color::Green),
                            Err(e) => self.set_message(format!("Error: {}", e), Color::Red),
                        }
                    } else {
                        self.set_message(
                            format!("Drone {} is already running", drone_name),
                            Color::Yellow,
                        );
                    }
                }
            }
            _ => {
                // Any other key cancels pending clean
                if self.pending_clean.is_some() {
                    self.pending_clean = None;
                    self.set_message("Clean cancelled".to_string(), Color::DarkGray);
                }
            }
        }

        Ok(KeyAction::Continue)
    }
}
