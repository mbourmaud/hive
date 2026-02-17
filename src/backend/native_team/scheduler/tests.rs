use std::collections::HashSet;

use crate::types::{StructuredTask, TaskType};

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
    let scheduler = TaskScheduler::new(tasks, 3, &HashSet::new());
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
    let scheduler = TaskScheduler::new(tasks, 3, &HashSet::new());
    let ready = scheduler.ready_tasks();
    // Task 2 is blocked by task 1
    assert_eq!(ready.len(), 2);
    assert!(ready.iter().any(|t| t.number == 1));
    assert!(ready.iter().any(|t| t.number == 3));
}

#[test]
fn test_mark_completed_unblocks_deps() {
    let tasks = vec![make_task(1, vec![], true), make_task(2, vec![1], true)];
    let mut scheduler = TaskScheduler::new(tasks, 3, &HashSet::new());

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
    let scheduler = TaskScheduler::new(tasks, 2, &HashSet::new());
    let ready = scheduler.ready_tasks();
    assert_eq!(ready.len(), 2);
}

#[test]
fn test_non_parallel_waits_for_empty() {
    let tasks = vec![
        make_task(1, vec![], true),
        make_task(2, vec![], false), // Non-parallel
    ];
    let mut scheduler = TaskScheduler::new(tasks, 3, &HashSet::new());

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
    let mut scheduler = TaskScheduler::new(tasks, 3, &HashSet::new());
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

    let scheduler = TaskScheduler::new(vec![setup, work], 3, &HashSet::new());
    assert_eq!(scheduler.task_count(), 1);
}

#[test]
fn test_requeue_failed_task() {
    let tasks = vec![make_task(1, vec![], true)];
    let mut scheduler = TaskScheduler::new(tasks, 3, &HashSet::new());

    scheduler.mark_running(1);
    scheduler.mark_failed(1);
    assert!(scheduler.has_failures());

    assert!(scheduler.requeue(1));
    let ready = scheduler.ready_tasks();
    assert_eq!(ready.len(), 1);
}

#[test]
fn test_requeue_respects_max_retries() {
    let tasks = vec![make_task(1, vec![], true)];
    let mut scheduler = TaskScheduler::new(tasks, 3, &HashSet::new());

    // First attempt + 2 retries = 3 total attempts
    for _ in 0..MAX_RETRIES {
        scheduler.mark_running(1);
        scheduler.mark_failed(1);
        assert!(scheduler.requeue(1));
    }

    // Third failure should NOT requeue
    scheduler.mark_running(1);
    scheduler.mark_failed(1);
    assert!(!scheduler.requeue(1));
    assert!(scheduler.has_failures());
    assert!(scheduler.ready_tasks().is_empty());
}

#[test]
fn test_resume_skips_completed_tasks() {
    let tasks = vec![
        make_task(1, vec![], true),
        make_task(2, vec![1], true),
        make_task(3, vec![1], true),
    ];
    // Task 1 was completed in a previous run
    let completed = HashSet::from([1]);
    let scheduler = TaskScheduler::new(tasks, 3, &completed);

    // Task 1 is already completed, tasks 2 and 3 are immediately ready
    assert!(!scheduler.all_completed());
    let ready = scheduler.ready_tasks();
    assert_eq!(ready.len(), 2);
    assert!(ready.iter().any(|t| t.number == 2));
    assert!(ready.iter().any(|t| t.number == 3));
}

#[test]
fn test_resume_all_completed() {
    let tasks = vec![make_task(1, vec![], true), make_task(2, vec![], true)];
    let completed = HashSet::from([1, 2]);
    let scheduler = TaskScheduler::new(tasks, 3, &completed);
    assert!(scheduler.all_completed());
}

#[test]
fn test_retry_count() {
    let tasks = vec![make_task(1, vec![], true)];
    let mut scheduler = TaskScheduler::new(tasks, 3, &HashSet::new());

    assert_eq!(scheduler.retry_count(1), 0);

    scheduler.mark_running(1);
    scheduler.mark_failed(1);
    scheduler.requeue(1);
    assert_eq!(scheduler.retry_count(1), 1);

    scheduler.mark_running(1);
    scheduler.mark_failed(1);
    scheduler.requeue(1);
    assert_eq!(scheduler.retry_count(1), 2);

    // Non-existent task returns 0
    assert_eq!(scheduler.retry_count(999), 0);
}
