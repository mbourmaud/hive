use crate::types::{StructuredTask, TaskType};

/// Status of a task in the scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Pending,
    Running,
    Completed,
    Failed,
}

/// A task wrapped with scheduling metadata.
#[derive(Debug)]
struct ScheduledTask {
    task: StructuredTask,
    state: TaskState,
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
    pub fn new(tasks: Vec<StructuredTask>, max_concurrent: usize) -> Self {
        let scheduled = tasks
            .into_iter()
            .filter(|t| t.task_type == TaskType::Work)
            .map(|task| ScheduledTask {
                task,
                state: TaskState::Pending,
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
    pub fn requeue(&mut self, task_number: usize) {
        if let Some(st) = self.find_mut(task_number) {
            st.state = TaskState::Pending;
        }
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
mod tests {
    use super::*;

    fn make_task(number: usize, depends_on: Vec<usize>, parallel: bool) -> StructuredTask {
        StructuredTask {
            number,
            title: format!("Task {number}"),
            body: String::new(),
            task_type: TaskType::Work,
            model: None,
            parallel,
            files: Vec::new(),
            depends_on,
        }
    }

    #[test]
    fn test_ready_tasks_no_deps() {
        let tasks = vec![make_task(1, vec![], true), make_task(2, vec![], true)];
        let scheduler = TaskScheduler::new(tasks, 3);
        let ready = scheduler.ready_tasks();
        assert_eq!(ready.len(), 2);
    }

    #[test]
    fn test_ready_tasks_with_deps() {
        let tasks = vec![
            make_task(1, vec![], true),
            make_task(2, vec![1], true),
            make_task(3, vec![], true),
        ];
        let scheduler = TaskScheduler::new(tasks, 3);
        let ready = scheduler.ready_tasks();
        // Task 2 is blocked by task 1
        assert_eq!(ready.len(), 2);
        assert!(ready.iter().any(|t| t.number == 1));
        assert!(ready.iter().any(|t| t.number == 3));
    }

    #[test]
    fn test_mark_completed_unblocks_deps() {
        let tasks = vec![make_task(1, vec![], true), make_task(2, vec![1], true)];
        let mut scheduler = TaskScheduler::new(tasks, 3);

        scheduler.mark_running(1);
        assert!(scheduler.ready_tasks().is_empty());

        scheduler.mark_completed(1);
        let ready = scheduler.ready_tasks();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].number, 2);
    }

    #[test]
    fn test_max_concurrent_respected() {
        let tasks = vec![
            make_task(1, vec![], true),
            make_task(2, vec![], true),
            make_task(3, vec![], true),
        ];
        let scheduler = TaskScheduler::new(tasks, 2);
        let ready = scheduler.ready_tasks();
        assert_eq!(ready.len(), 2);
    }

    #[test]
    fn test_non_parallel_waits_for_empty() {
        let tasks = vec![
            make_task(1, vec![], true),
            make_task(2, vec![], false), // Non-parallel
        ];
        let mut scheduler = TaskScheduler::new(tasks, 3);

        // With nothing running, non-parallel can start (but only alone)
        let ready = scheduler.ready_tasks();
        // Task 1 (parallel) gets picked first, then task 2 won't run with others
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].number, 1);

        scheduler.mark_running(1);
        scheduler.mark_completed(1);

        let ready = scheduler.ready_tasks();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].number, 2);
    }

    #[test]
    fn test_all_completed() {
        let tasks = vec![make_task(1, vec![], true), make_task(2, vec![], true)];
        let mut scheduler = TaskScheduler::new(tasks, 3);
        assert!(!scheduler.all_completed());

        scheduler.mark_running(1);
        scheduler.mark_completed(1);
        assert!(!scheduler.all_completed());

        scheduler.mark_running(2);
        scheduler.mark_completed(2);
        assert!(scheduler.all_completed());
    }

    #[test]
    fn test_filters_non_work_tasks() {
        let mut setup = make_task(1, vec![], true);
        setup.task_type = TaskType::Setup;
        let work = make_task(2, vec![], true);

        let scheduler = TaskScheduler::new(vec![setup, work], 3);
        assert_eq!(scheduler.task_count(), 1);
    }

    #[test]
    fn test_requeue_failed_task() {
        let tasks = vec![make_task(1, vec![], true)];
        let mut scheduler = TaskScheduler::new(tasks, 3);

        scheduler.mark_running(1);
        scheduler.mark_failed(1);
        assert!(scheduler.has_failures());

        scheduler.requeue(1);
        let ready = scheduler.ready_tasks();
        assert_eq!(ready.len(), 1);
    }
}
