package tasks

import (
	"encoding/json"
	"fmt"
	"sync"
	"time"
)

// TaskStatus represents the current state of a task
type TaskStatus string

const (
	TaskStatusPending    TaskStatus = "pending"
	TaskStatusAssigned   TaskStatus = "assigned"
	TaskStatusInProgress TaskStatus = "in_progress"
	TaskStatusCompleted  TaskStatus = "completed"
	TaskStatusFailed     TaskStatus = "failed"
	TaskStatusTimedOut   TaskStatus = "timed_out"
)

// Task represents a unit of work to be executed by a worker
type Task struct {
	ID          string                 `json:"id"`
	Description string                 `json:"description"`
	Status      TaskStatus             `json:"status"`
	WorkerID    string                 `json:"worker_id,omitempty"`
	CreatedAt   time.Time              `json:"created_at"`
	AssignedAt  *time.Time             `json:"assigned_at,omitempty"`
	StartedAt   *time.Time             `json:"started_at,omitempty"`
	CompletedAt *time.Time             `json:"completed_at,omitempty"`
	Timeout     time.Duration          `json:"timeout"`
	Retries     int                    `json:"retries"`
	MaxRetries  int                    `json:"max_retries"`
	Metadata    map[string]interface{} `json:"metadata,omitempty"`
	Result      string                 `json:"result,omitempty"`
	Error       string                 `json:"error,omitempty"`
}

// Worker represents an agent that can execute tasks
type Worker struct {
	ID            string    `json:"id"`
	Status        string    `json:"status"`
	CurrentTask   string    `json:"current_task,omitempty"`
	LastHeartbeat time.Time `json:"last_heartbeat"`
	TasksComplete int       `json:"tasks_complete"`
	TasksFailed   int       `json:"tasks_failed"`
}

// ManagerConfig holds configuration for the task manager
type ManagerConfig struct {
	HeartbeatInterval time.Duration // How often workers should send heartbeats
	HeartbeatTimeout  time.Duration // How long before a worker is considered dead
	DefaultTimeout    time.Duration // Default task timeout
	MaxRetries        int           // Default max retries for failed tasks
}

// DefaultConfig returns a default configuration
func DefaultConfig() ManagerConfig {
	return ManagerConfig{
		HeartbeatInterval: 30 * time.Second,
		HeartbeatTimeout:  90 * time.Second,
		DefaultTimeout:    30 * time.Minute,
		MaxRetries:        3,
	}
}

// Manager handles task queue operations
type Manager struct {
	mu      sync.RWMutex
	config  ManagerConfig
	tasks   map[string]*Task
	workers map[string]*Worker
	queue   []string // Task IDs in queue order
}

// NewManager creates a new task manager
func NewManager(config ManagerConfig) *Manager {
	return &Manager{
		config:  config,
		tasks:   make(map[string]*Task),
		workers: make(map[string]*Worker),
		queue:   make([]string, 0),
	}
}

// CreateTask creates a new task and adds it to the queue
func (m *Manager) CreateTask(id, description string, timeout time.Duration) *Task {
	m.mu.Lock()
	defer m.mu.Unlock()

	if timeout == 0 {
		timeout = m.config.DefaultTimeout
	}

	task := &Task{
		ID:          id,
		Description: description,
		Status:      TaskStatusPending,
		CreatedAt:   time.Now(),
		Timeout:     timeout,
		MaxRetries:  m.config.MaxRetries,
		Metadata:    make(map[string]interface{}),
	}

	m.tasks[id] = task
	m.queue = append(m.queue, id)
	return task
}

// GetTask retrieves a task by ID
func (m *Manager) GetTask(id string) (*Task, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	task, ok := m.tasks[id]
	if !ok {
		return nil, fmt.Errorf("task not found: %s", id)
	}
	return task, nil
}

// AssignTask assigns a task to a worker
func (m *Manager) AssignTask(taskID, workerID string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	task, ok := m.tasks[taskID]
	if !ok {
		return fmt.Errorf("task not found: %s", taskID)
	}

	if task.Status != TaskStatusPending {
		return fmt.Errorf("task %s is not pending (status: %s)", taskID, task.Status)
	}

	worker, ok := m.workers[workerID]
	if !ok {
		return fmt.Errorf("worker not found: %s", workerID)
	}

	now := time.Now()
	task.Status = TaskStatusAssigned
	task.WorkerID = workerID
	task.AssignedAt = &now
	worker.CurrentTask = taskID

	// Remove from queue
	for i, id := range m.queue {
		if id == taskID {
			m.queue = append(m.queue[:i], m.queue[i+1:]...)
			break
		}
	}

	return nil
}

// StartTask marks a task as in progress
func (m *Manager) StartTask(taskID string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	task, ok := m.tasks[taskID]
	if !ok {
		return fmt.Errorf("task not found: %s", taskID)
	}

	if task.Status != TaskStatusAssigned {
		return fmt.Errorf("task %s is not assigned (status: %s)", taskID, task.Status)
	}

	now := time.Now()
	task.Status = TaskStatusInProgress
	task.StartedAt = &now
	return nil
}

// CompleteTask marks a task as completed
func (m *Manager) CompleteTask(taskID, result string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	task, ok := m.tasks[taskID]
	if !ok {
		return fmt.Errorf("task not found: %s", taskID)
	}

	now := time.Now()
	task.Status = TaskStatusCompleted
	task.CompletedAt = &now
	task.Result = result

	// Update worker stats
	if worker, ok := m.workers[task.WorkerID]; ok {
		worker.TasksComplete++
		worker.CurrentTask = ""
	}

	return nil
}

// FailTask marks a task as failed and optionally requeues it
func (m *Manager) FailTask(taskID, errMsg string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	task, ok := m.tasks[taskID]
	if !ok {
		return fmt.Errorf("task not found: %s", taskID)
	}

	task.Retries++
	task.Error = errMsg

	// Update worker stats
	if worker, ok := m.workers[task.WorkerID]; ok {
		worker.TasksFailed++
		worker.CurrentTask = ""
	}

	// Requeue if retries available
	if task.Retries < task.MaxRetries {
		task.Status = TaskStatusPending
		task.WorkerID = ""
		task.AssignedAt = nil
		task.StartedAt = nil
		m.queue = append(m.queue, taskID)
	} else {
		task.Status = TaskStatusFailed
		now := time.Now()
		task.CompletedAt = &now
	}

	return nil
}

// RegisterWorker registers a new worker
func (m *Manager) RegisterWorker(id string) *Worker {
	m.mu.Lock()
	defer m.mu.Unlock()

	worker := &Worker{
		ID:            id,
		Status:        "active",
		LastHeartbeat: time.Now(),
	}
	m.workers[id] = worker
	return worker
}

// Heartbeat updates a worker's last heartbeat time
func (m *Manager) Heartbeat(workerID string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	worker, ok := m.workers[workerID]
	if !ok {
		return fmt.Errorf("worker not found: %s", workerID)
	}

	worker.LastHeartbeat = time.Now()
	worker.Status = "active"
	return nil
}

// CheckTimeouts checks for timed out tasks and dead workers
func (m *Manager) CheckTimeouts() []string {
	m.mu.Lock()
	defer m.mu.Unlock()

	now := time.Now()
	var redistribted []string

	// Check for dead workers
	for _, worker := range m.workers {
		if now.Sub(worker.LastHeartbeat) > m.config.HeartbeatTimeout {
			worker.Status = "dead"

			// Redistribute the worker's task
			if worker.CurrentTask != "" {
				if task, ok := m.tasks[worker.CurrentTask]; ok {
					task.Status = TaskStatusTimedOut
					task.Retries++
					if task.Retries < task.MaxRetries {
						task.Status = TaskStatusPending
						task.WorkerID = ""
						task.AssignedAt = nil
						task.StartedAt = nil
						m.queue = append(m.queue, task.ID)
						redistribted = append(redistribted, task.ID)
					}
				}
				worker.CurrentTask = ""
			}
		}
	}

	// Check for timed out tasks
	for _, task := range m.tasks {
		if task.Status == TaskStatusInProgress && task.StartedAt != nil {
			if now.Sub(*task.StartedAt) > task.Timeout {
				task.Status = TaskStatusTimedOut
				task.Retries++

				if worker, ok := m.workers[task.WorkerID]; ok {
					worker.CurrentTask = ""
				}

				if task.Retries < task.MaxRetries {
					task.Status = TaskStatusPending
					task.WorkerID = ""
					task.AssignedAt = nil
					task.StartedAt = nil
					m.queue = append(m.queue, task.ID)
					redistribted = append(redistribted, task.ID)
				}
			}
		}
	}

	return redistribted
}

// GetNextPendingTask returns the next task from the queue
func (m *Manager) GetNextPendingTask() *Task {
	m.mu.RLock()
	defer m.mu.RUnlock()

	if len(m.queue) == 0 {
		return nil
	}

	taskID := m.queue[0]
	return m.tasks[taskID]
}

// GetWorkerStats returns statistics for all workers
func (m *Manager) GetWorkerStats() map[string]*Worker {
	m.mu.RLock()
	defer m.mu.RUnlock()

	stats := make(map[string]*Worker)
	for id, worker := range m.workers {
		stats[id] = worker
	}
	return stats
}

// GetQueueLength returns the number of pending tasks
func (m *Manager) GetQueueLength() int {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return len(m.queue)
}

// ToJSON serializes the task to JSON
func (t *Task) ToJSON() ([]byte, error) {
	return json.Marshal(t)
}

// TaskFromJSON deserializes a task from JSON
func TaskFromJSON(data []byte) (*Task, error) {
	var task Task
	if err := json.Unmarshal(data, &task); err != nil {
		return nil, err
	}
	return &task, nil
}
