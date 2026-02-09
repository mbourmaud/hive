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
                if self.messages_view.is_some() {
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
                    self.message = Some(msg);
                    self.message_color = Color::Green;
                }
                Ok(None) => {}
                Err(e) => {
                    self.message = Some(format!("Error: {}", e));
                    self.message_color = Color::Red;
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
            KeyCode::Char('x') | KeyCode::Char('X') => {
                if !self.drones.is_empty() {
                    let drone_name = self.drones[current_drone_idx].0.clone();
                    match handle_stop_drone(&drone_name) {
                        Ok(msg) => {
                            self.message = Some(msg);
                            self.message_color = Color::Green;
                        }
                        Err(e) => {
                            self.message = Some(format!("Error: {}", e));
                            self.message_color = Color::Red;
                        }
                    }
                }
            }
            KeyCode::Char('D') => {
                if !self.drones.is_empty() {
                    let drone_name = self.drones[current_drone_idx].0.clone();
                    if let Some((ref pending, when)) = self.pending_clean {
                        if pending == &drone_name && when.elapsed() < Duration::from_secs(5) {
                            // Second D within 5 seconds = confirmed
                            self.pending_clean = None;
                            match handle_clean_drone(&drone_name) {
                                Ok(msg) => {
                                    self.message = Some(msg);
                                    self.message_color = Color::Green;
                                }
                                Err(e) => {
                                    self.message = Some(format!("Error: {}", e));
                                    self.message_color = Color::Red;
                                }
                            }
                        } else {
                            // Different drone or timeout — reset
                            self.pending_clean = Some((drone_name.clone(), Instant::now()));
                            self.message = Some(format!(
                                "Press D again to confirm cleaning '{}'",
                                drone_name
                            ));
                            self.message_color = Color::Yellow;
                        }
                    } else {
                        // First D — prompt
                        self.pending_clean = Some((drone_name.clone(), Instant::now()));
                        self.message = Some(format!(
                            "Press D again to confirm cleaning '{}'",
                            drone_name
                        ));
                        self.message_color = Color::Yellow;
                    }
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                if !self.drones.is_empty() {
                    let drone_name = self.drones[current_drone_idx].0.clone();
                    let status = &self.drones[current_drone_idx].1;

                    if status.status == DroneState::Completed
                        || status.status == DroneState::Stopped
                    {
                        match handle_resume_drone(&drone_name) {
                            Ok(msg) => {
                                self.message = Some(msg);
                                self.message_color = Color::Green;
                            }
                            Err(e) => {
                                self.message = Some(format!("Error: {}", e));
                                self.message_color = Color::Red;
                            }
                        }
                    } else {
                        self.message = Some(format!("Drone {} is already running", drone_name));
                        self.message_color = Color::Yellow;
                    }
                }
            }
            _ => {}
        }

        Ok(KeyAction::Continue)
    }
}
