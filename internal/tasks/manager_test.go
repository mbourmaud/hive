package tasks

import (
	"testing"
	"time"
)

func TestNewManager(t *testing.T) {
	config := DefaultConfig()
	m := NewManager(config)

	if m == nil {
		t.Fatal("NewManager returned nil")
	}
	if m.GetQueueLength() != 0 {
		t.Errorf("expected empty queue, got %d", m.GetQueueLength())
	}
}

func TestCreateTask(t *testing.T) {
	m := NewManager(DefaultConfig())

	task := m.CreateTask("task-1", "Test task", 10*time.Minute)

	if task.ID != "task-1" {
		t.Errorf("expected ID 'task-1', got '%s'", task.ID)
	}
	if task.Description != "Test task" {
		t.Errorf("expected description 'Test task', got '%s'", task.Description)
	}
	if task.Status != TaskStatusPending {
		t.Errorf("expected status PENDING, got '%s'", task.Status)
	}
	if task.Timeout != 10*time.Minute {
		t.Errorf("expected timeout 10m, got %v", task.Timeout)
	}
	if m.GetQueueLength() != 1 {
		t.Errorf("expected queue length 1, got %d", m.GetQueueLength())
	}
}

func TestGetTask(t *testing.T) {
	m := NewManager(DefaultConfig())
	m.CreateTask("task-1", "Test task", 0)

	task, err := m.GetTask("task-1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if task.ID != "task-1" {
		t.Errorf("expected ID 'task-1', got '%s'", task.ID)
	}

	_, err = m.GetTask("nonexistent")
	if err == nil {
		t.Error("expected error for nonexistent task")
	}
}

func TestRegisterWorker(t *testing.T) {
	m := NewManager(DefaultConfig())

	worker := m.RegisterWorker("worker-1")

	if worker.ID != "worker-1" {
		t.Errorf("expected ID 'worker-1', got '%s'", worker.ID)
	}
	if worker.Status != "active" {
		t.Errorf("expected status 'active', got '%s'", worker.Status)
	}
}

func TestAssignTask(t *testing.T) {
	m := NewManager(DefaultConfig())
	m.CreateTask("task-1", "Test task", 0)
	m.RegisterWorker("worker-1")

	err := m.AssignTask("task-1", "worker-1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	task, _ := m.GetTask("task-1")
	if task.Status != TaskStatusAssigned {
		t.Errorf("expected status ASSIGNED, got '%s'", task.Status)
	}
	if task.WorkerID != "worker-1" {
		t.Errorf("expected WorkerID 'worker-1', got '%s'", task.WorkerID)
	}
	if m.GetQueueLength() != 0 {
		t.Errorf("expected queue length 0, got %d", m.GetQueueLength())
	}
}

func TestStartTask(t *testing.T) {
	m := NewManager(DefaultConfig())
	m.CreateTask("task-1", "Test task", 0)
	m.RegisterWorker("worker-1")
	m.AssignTask("task-1", "worker-1")

	err := m.StartTask("task-1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	task, _ := m.GetTask("task-1")
	if task.Status != TaskStatusInProgress {
		t.Errorf("expected status IN_PROGRESS, got '%s'", task.Status)
	}
	if task.StartedAt == nil {
		t.Error("StartedAt should be set")
	}
}

func TestCompleteTask(t *testing.T) {
	m := NewManager(DefaultConfig())
	m.CreateTask("task-1", "Test task", 0)
	m.RegisterWorker("worker-1")
	m.AssignTask("task-1", "worker-1")
	m.StartTask("task-1")

	err := m.CompleteTask("task-1", "Success!")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	task, _ := m.GetTask("task-1")
	if task.Status != TaskStatusCompleted {
		t.Errorf("expected status COMPLETED, got '%s'", task.Status)
	}
	if task.Result != "Success!" {
		t.Errorf("expected result 'Success!', got '%s'", task.Result)
	}

	stats := m.GetWorkerStats()
	if stats["worker-1"].TasksComplete != 1 {
		t.Errorf("expected TasksComplete 1, got %d", stats["worker-1"].TasksComplete)
	}
}

func TestFailTaskWithRetry(t *testing.T) {
	config := DefaultConfig()
	config.MaxRetries = 3
	m := NewManager(config)

	m.CreateTask("task-1", "Test task", 0)
	m.RegisterWorker("worker-1")
	m.AssignTask("task-1", "worker-1")
	m.StartTask("task-1")

	err := m.FailTask("task-1", "Something went wrong")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	task, _ := m.GetTask("task-1")
	if task.Status != TaskStatusPending {
		t.Errorf("expected status PENDING (retry), got '%s'", task.Status)
	}
	if task.Retries != 1 {
		t.Errorf("expected Retries 1, got %d", task.Retries)
	}
	if m.GetQueueLength() != 1 {
		t.Errorf("expected queue length 1, got %d", m.GetQueueLength())
	}
}

func TestFailTaskNoRetry(t *testing.T) {
	config := DefaultConfig()
	config.MaxRetries = 1
	m := NewManager(config)

	m.CreateTask("task-1", "Test task", 0)
	m.RegisterWorker("worker-1")
	m.AssignTask("task-1", "worker-1")
	m.StartTask("task-1")

	m.FailTask("task-1", "Error 1")
	task, _ := m.GetTask("task-1")
	if task.Status != TaskStatusFailed {
		t.Errorf("expected status FAILED, got '%s'", task.Status)
	}
}

func TestHeartbeat(t *testing.T) {
	m := NewManager(DefaultConfig())
	m.RegisterWorker("worker-1")

	time.Sleep(10 * time.Millisecond)
	err := m.Heartbeat("worker-1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	stats := m.GetWorkerStats()
	if stats["worker-1"].Status != "active" {
		t.Errorf("expected status 'active', got '%s'", stats["worker-1"].Status)
	}
}

func TestCheckTimeoutsDeadWorker(t *testing.T) {
	config := DefaultConfig()
	config.HeartbeatTimeout = 10 * time.Millisecond
	config.MaxRetries = 3
	m := NewManager(config)

	m.CreateTask("task-1", "Test task", 0)
	m.RegisterWorker("worker-1")
	m.AssignTask("task-1", "worker-1")
	m.StartTask("task-1")

	// Simulate worker going dead
	time.Sleep(20 * time.Millisecond)

	redistributed := m.CheckTimeouts()
	if len(redistributed) != 1 {
		t.Errorf("expected 1 redistributed task, got %d", len(redistributed))
	}

	task, _ := m.GetTask("task-1")
	if task.Status != TaskStatusPending {
		t.Errorf("expected status PENDING, got '%s'", task.Status)
	}

	stats := m.GetWorkerStats()
	if stats["worker-1"].Status != "dead" {
		t.Errorf("expected worker status 'dead', got '%s'", stats["worker-1"].Status)
	}
}

func TestCheckTimeoutsTaskTimeout(t *testing.T) {
	config := DefaultConfig()
	config.MaxRetries = 3
	m := NewManager(config)

	m.CreateTask("task-1", "Test task", 10*time.Millisecond)
	m.RegisterWorker("worker-1")
	m.AssignTask("task-1", "worker-1")
	m.StartTask("task-1")

	// Keep worker alive
	m.Heartbeat("worker-1")

	// Wait for task to timeout
	time.Sleep(20 * time.Millisecond)

	redistributed := m.CheckTimeouts()
	if len(redistributed) != 1 {
		t.Errorf("expected 1 redistributed task, got %d", len(redistributed))
	}

	task, _ := m.GetTask("task-1")
	if task.Status != TaskStatusPending {
		t.Errorf("expected status PENDING, got '%s'", task.Status)
	}
}

func TestGetNextPendingTask(t *testing.T) {
	m := NewManager(DefaultConfig())

	// Empty queue
	task := m.GetNextPendingTask()
	if task != nil {
		t.Error("expected nil for empty queue")
	}

	m.CreateTask("task-1", "First task", 0)
	m.CreateTask("task-2", "Second task", 0)

	task = m.GetNextPendingTask()
	if task.ID != "task-1" {
		t.Errorf("expected 'task-1', got '%s'", task.ID)
	}
}

func TestTaskJSON(t *testing.T) {
	task := &Task{
		ID:          "task-1",
		Description: "Test task",
		Status:      TaskStatusPending,
		CreatedAt:   time.Now(),
		Timeout:     10 * time.Minute,
	}

	data, err := task.ToJSON()
	if err != nil {
		t.Fatalf("ToJSON failed: %v", err)
	}

	parsed, err := TaskFromJSON(data)
	if err != nil {
		t.Fatalf("TaskFromJSON failed: %v", err)
	}

	if parsed.ID != task.ID {
		t.Errorf("expected ID '%s', got '%s'", task.ID, parsed.ID)
	}
	if parsed.Description != task.Description {
		t.Errorf("expected Description '%s', got '%s'", task.Description, parsed.Description)
	}
}

func TestDefaultConfig(t *testing.T) {
	config := DefaultConfig()

	if config.HeartbeatInterval != 30*time.Second {
		t.Errorf("expected HeartbeatInterval 30s, got %v", config.HeartbeatInterval)
	}
	if config.HeartbeatTimeout != 90*time.Second {
		t.Errorf("expected HeartbeatTimeout 90s, got %v", config.HeartbeatTimeout)
	}
	if config.DefaultTimeout != 30*time.Minute {
		t.Errorf("expected DefaultTimeout 30m, got %v", config.DefaultTimeout)
	}
	if config.MaxRetries != 3 {
		t.Errorf("expected MaxRetries 3, got %d", config.MaxRetries)
	}
}
