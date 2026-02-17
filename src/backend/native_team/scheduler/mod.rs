use std::collections::HashSet;

use crate::types::{StructuredTask, TaskType};

/// Status of a task in the scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Maximum number of retries before a task is permanently failed.
const MAX_RETRIES: usize = 2;

/// A task wrapped with scheduling metadata.
#[derive(Debug)]
struct ScheduledTask {
    task: StructuredTask,
    state: TaskState,
    retries: usize,
}

/// DAG-based task scheduler with dependency resolution.
///
/// Resolves which tasks are ready to run based on their `depends_on`
/// relationships, respects `parallel` flags, and enforces `max_concurrent`.
pub struct TaskScheduler {
    tasks: Vec<ScheduledTask>,
    max_concurrent: usize,
}

impl TaskScheduler {
    /// Create a new scheduler from structured tasks (Work tasks only).
    ///
    /// `completed_numbers` contains plan task numbers that were already
    /// completed in a previous run — these are initialized as `Completed`
    /// so the scheduler skips them (resume support).
    pub fn new(
        tasks: Vec<StructuredTask>,
        max_concurrent: usize,
        completed_numbers: &HashSet<usize>,
    ) -> Self {
        let scheduled = tasks
            .into_iter()
            .filter(|t| t.task_type == TaskType::Work)
            .map(|task| {
                let state = if completed_numbers.contains(&task.number) {
                    TaskState::Completed
                } else {
                    TaskState::Pending
                };
                ScheduledTask {
                    task,
                    state,
                    retries: 0,
                }
            })
            .collect();
        Self {
            tasks: scheduled,
            max_concurrent,
        }
    }

    /// Return tasks that are ready to run: pending, all deps completed,
    /// respects parallel flag and max_concurrent limit.
    pub fn ready_tasks(&self) -> Vec<&StructuredTask> {
        let running_count = self
            .tasks
            .iter()
            .filter(|t| t.state == TaskState::Running)
            .count();
        let available_slots = self.max_concurrent.saturating_sub(running_count);

        if available_slots == 0 {
            return Vec::new();
        }

        let mut ready = Vec::new();
        for st in &self.tasks {
            if st.state != TaskState::Pending {
                continue;
            }
            if !self.deps_completed(st.task.number) {
                continue;
            }
            // Non-parallel tasks: only run when nothing else is running
            if !st.task.parallel && running_count + ready.len() > 0 {
                continue;
            }
            ready.push(&st.task);
            if ready.len() >= available_slots {
                break;
            }
        }

        ready
    }

    pub fn mark_running(&mut self, task_number: usize) {
        if let Some(st) = self.find_mut(task_number) {
            st.state = TaskState::Running;
        }
    }

    pub fn mark_completed(&mut self, task_number: usize) {
        if let Some(st) = self.find_mut(task_number) {
            st.state = TaskState::Completed;
        }
    }

    pub fn mark_failed(&mut self, task_number: usize) {
        if let Some(st) = self.find_mut(task_number) {
            st.state = TaskState::Failed;
        }
    }

    /// Re-queue a failed task as pending for retry.
    /// Returns `false` if the task has exceeded its retry limit.
    pub fn requeue(&mut self, task_number: usize) -> bool {
        if let Some(st) = self.find_mut(task_number) {
            if st.retries >= MAX_RETRIES {
                return false;
            }
            st.retries += 1;
            st.state = TaskState::Pending;
            return true;
        }
        false
    }

    /// Return the number of times a task has been retried.
    pub fn retry_count(&self, task_number: usize) -> usize {
        self.tasks
            .iter()
            .find(|st| st.task.number == task_number)
            .map(|st| st.retries)
            .unwrap_or(0)
    }

    pub fn all_completed(&self) -> bool {
        self.tasks.iter().all(|t| t.state == TaskState::Completed)
    }

    pub fn has_failures(&self) -> bool {
        self.tasks.iter().any(|t| t.state == TaskState::Failed)
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn get_task(&self, task_number: usize) -> Option<&StructuredTask> {
        self.tasks
            .iter()
            .find(|st| st.task.number == task_number)
            .map(|st| &st.task)
    }

    /// Check if all dependencies for a task are completed.
    fn deps_completed(&self, task_number: usize) -> bool {
        let Some(st) = self.tasks.iter().find(|t| t.task.number == task_number) else {
            return true;
        };
        st.task.depends_on.iter().all(|dep| {
            self.tasks
                .iter()
                .find(|t| t.task.number == *dep)
                .map(|t| t.state == TaskState::Completed)
                .unwrap_or(true) // Missing dep = external, assume completed
        })
    }

    fn find_mut(&mut self, task_number: usize) -> Option<&mut ScheduledTask> {
        self.tasks.iter_mut().find(|t| t.task.number == task_number)
    }
}

#[cfg(test)]
mod tests;
