package task

import (
	"fmt"
	"sync"
	"time"

	"github.com/google/uuid"
	"github.com/mbourmaud/hive/internal/event"
)

// EventHandler is called when task events occur
type EventHandler func(event TaskEvent)

// Manager manages tasks and their lifecycle
type Manager struct {
	mu         sync.RWMutex
	tasks      map[string]*Task
	byAgent    map[string][]string // agentID -> taskIDs
	dispatcher *event.Dispatcher[TaskEvent]
}

// NewManager creates a new task manager
func NewManager(handler EventHandler) *Manager {
	m := &Manager{
		tasks:   make(map[string]*Task),
		byAgent: make(map[string][]string),
	}
	if handler != nil {
		m.dispatcher = event.NewDispatcher(handler, 4, 100)
		m.dispatcher.Start()
	}
	return m
}

// Close shuts down the manager and its event dispatcher.
func (m *Manager) Close() {
	if m.dispatcher != nil {
		m.dispatcher.Stop()
	}
}

// Create creates a new task from a request
func (m *Manager) Create(req CreateTaskRequest) (*Task, error) {
	if req.AgentID == "" {
		return nil, fmt.Errorf("agent_id is required")
	}
	if req.Title == "" {
		return nil, fmt.Errorf("title is required")
	}
	if len(req.Steps) == 0 {
		return nil, fmt.Errorf("at least one step is required")
	}

	// Build steps
	steps := make([]Step, len(req.Steps))
	for i, s := range req.Steps {
		if s.Action == "" {
			return nil, fmt.Errorf("step %d: action is required", i+1)
		}
		if len(s.DoD) == 0 {
			return nil, fmt.Errorf("step %d: at least one DoD item is required", i+1)
		}

		autonomy := s.Autonomy
		if autonomy == "" {
			autonomy = AutonomyAskIfUnclear
		}

		steps[i] = Step{
			ID:          i + 1,
			Action:      s.Action,
			Description: s.Description,
			DoD:         s.DoD,
			Autonomy:    autonomy,
			Status:      StepStatusPending,
		}
	}

	// Create plan
	plan := Plan{
		ID:            uuid.New().String(),
		Title:         req.Title,
		Description:   req.Description,
		Context:       req.Context,
		Steps:         steps,
		RequiredPorts: req.RequiredPorts,
		OnBlocker:     req.OnBlocker,
		OnAmbiguity:   req.OnAmbiguity,
		OnComplete:    req.OnComplete,
		CreatedAt:     time.Now(),
		CreatedBy:     "queen",
	}

	// Create task
	task := &Task{
		ID:          uuid.New().String(),
		AgentID:     req.AgentID,
		AgentName:   req.AgentName,
		Plan:        plan,
		Status:      TaskStatusAssigned,
		CurrentStep: 0,
		CreatedAt:   time.Now(),
	}

	m.mu.Lock()
	m.tasks[task.ID] = task
	m.byAgent[req.AgentID] = append(m.byAgent[req.AgentID], task.ID)
	m.mu.Unlock()

	m.emitEvent(TaskEvent{
		Type:      "created",
		Task:      task,
		Timestamp: time.Now(),
	})

	return task, nil
}

// Get returns a task by ID
func (m *Manager) Get(id string) (*Task, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	task, ok := m.tasks[id]
	if !ok {
		return nil, fmt.Errorf("task %s not found", id)
	}
	return task, nil
}

// List returns all tasks, optionally filtered
func (m *Manager) List(agentID string, status TaskStatus) []*Task {
	m.mu.RLock()
	defer m.mu.RUnlock()

	tasks := make([]*Task, 0)
	for _, task := range m.tasks {
		if agentID != "" && task.AgentID != agentID {
			continue
		}
		if status != "" && task.Status != status {
			continue
		}
		tasks = append(tasks, task)
	}
	return tasks
}

// GetByAgent returns the current task for an agent
func (m *Manager) GetByAgent(agentID string) *Task {
	m.mu.RLock()
	defer m.mu.RUnlock()

	taskIDs, ok := m.byAgent[agentID]
	if !ok || len(taskIDs) == 0 {
		return nil
	}

	// Return the most recent active task
	for i := len(taskIDs) - 1; i >= 0; i-- {
		task := m.tasks[taskIDs[i]]
		if task != nil && task.Status != TaskStatusCompleted &&
			task.Status != TaskStatusFailed && task.Status != TaskStatusCancelled {
			return task
		}
	}
	return nil
}

// Start marks a task as started
func (m *Manager) Start(taskID string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	task, ok := m.tasks[taskID]
	if !ok {
		return fmt.Errorf("task %s not found", taskID)
	}

	if task.Status != TaskStatusAssigned {
		return fmt.Errorf("task %s is not in assigned status", taskID)
	}

	now := time.Now()
	task.Status = TaskStatusInProgress
	task.StartedAt = &now
	task.CurrentStep = 1

	// Mark first step as in progress
	if len(task.Plan.Steps) > 0 {
		task.Plan.Steps[0].Status = StepStatusInProgress
		task.Plan.Steps[0].StartedAt = &now
	}

	m.emitEvent(TaskEvent{
		Type:      "started",
		Task:      task,
		Timestamp: now,
	})

	return nil
}

// UpdateStep updates a step's status
func (m *Manager) UpdateStep(taskID string, stepID int, req UpdateStepRequest) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	task, ok := m.tasks[taskID]
	if !ok {
		return fmt.Errorf("task %s not found", taskID)
	}

	if stepID < 1 || stepID > len(task.Plan.Steps) {
		return fmt.Errorf("invalid step ID: %d", stepID)
	}

	step := &task.Plan.Steps[stepID-1]
	step.Status = req.Status
	step.Result = req.Result
	step.Error = req.Error

	now := time.Now()

	switch req.Status {
	case StepStatusInProgress:
		step.StartedAt = &now
	case StepStatusCompleted, StepStatusFailed, StepStatusSkipped:
		step.CompletedAt = &now
	case StepStatusWaiting, StepStatusBlocked:
		task.Status = TaskStatusWaiting
	}

	// If step completed, move to next or complete task
	if req.Status == StepStatusCompleted {
		if stepID < len(task.Plan.Steps) {
			task.CurrentStep = stepID + 1
			nextStep := &task.Plan.Steps[stepID]
			nextStep.Status = StepStatusInProgress
			nextStep.StartedAt = &now
			task.Status = TaskStatusInProgress
		}
	}

	m.emitEvent(TaskEvent{
		Type:      "progress",
		Task:      task,
		Step:      step,
		Progress:  m.calculateProgress(task),
		Timestamp: now,
	})

	return nil
}

// Complete marks a task as completed
func (m *Manager) Complete(taskID string, req CompleteTaskRequest) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	task, ok := m.tasks[taskID]
	if !ok {
		return fmt.Errorf("task %s not found", taskID)
	}

	now := time.Now()
	task.Status = TaskStatusCompleted
	task.Result = req.Result
	task.Artifacts = req.Artifacts
	task.CompletedAt = &now

	// Mark current step as completed if not already
	if task.CurrentStep > 0 && task.CurrentStep <= len(task.Plan.Steps) {
		step := &task.Plan.Steps[task.CurrentStep-1]
		if step.Status != StepStatusCompleted {
			step.Status = StepStatusCompleted
			step.CompletedAt = &now
		}
	}

	m.emitEvent(TaskEvent{
		Type:      "completed",
		Task:      task,
		Progress:  100,
		Timestamp: now,
	})

	return nil
}

// Fail marks a task as failed
func (m *Manager) Fail(taskID string, req FailTaskRequest) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	task, ok := m.tasks[taskID]
	if !ok {
		return fmt.Errorf("task %s not found", taskID)
	}

	now := time.Now()
	task.Status = TaskStatusFailed
	task.Error = req.Error
	task.CompletedAt = &now

	// Mark current step as failed
	if task.CurrentStep > 0 && task.CurrentStep <= len(task.Plan.Steps) {
		step := &task.Plan.Steps[task.CurrentStep-1]
		step.Status = StepStatusFailed
		step.Error = req.Error
		step.CompletedAt = &now
	}

	m.emitEvent(TaskEvent{
		Type:      "failed",
		Task:      task,
		Message:   req.Error,
		Timestamp: now,
	})

	return nil
}

// Cancel cancels a task
func (m *Manager) Cancel(taskID string, reason string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	task, ok := m.tasks[taskID]
	if !ok {
		return fmt.Errorf("task %s not found", taskID)
	}

	now := time.Now()
	task.Status = TaskStatusCancelled
	task.Error = reason
	task.CompletedAt = &now

	m.emitEvent(TaskEvent{
		Type:      "cancelled",
		Task:      task,
		Message:   reason,
		Timestamp: now,
	})

	return nil
}

// Delete removes a task from the manager
func (m *Manager) Delete(taskID string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	task, ok := m.tasks[taskID]
	if !ok {
		return fmt.Errorf("task %s not found", taskID)
	}

	// Remove from byAgent
	agentTasks := m.byAgent[task.AgentID]
	filtered := make([]string, 0, len(agentTasks))
	for _, id := range agentTasks {
		if id != taskID {
			filtered = append(filtered, id)
		}
	}
	if len(filtered) > 0 {
		m.byAgent[task.AgentID] = filtered
	} else {
		delete(m.byAgent, task.AgentID)
	}

	delete(m.tasks, taskID)
	return nil
}

// GetCurrentStep returns the current step for a task
func (m *Manager) GetCurrentStep(taskID string) (*Step, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	task, ok := m.tasks[taskID]
	if !ok {
		return nil, fmt.Errorf("task %s not found", taskID)
	}

	if task.CurrentStep < 1 || task.CurrentStep > len(task.Plan.Steps) {
		return nil, fmt.Errorf("no current step")
	}

	return &task.Plan.Steps[task.CurrentStep-1], nil
}

// calculateProgress returns the completion percentage of a task
func (m *Manager) calculateProgress(task *Task) int {
	if len(task.Plan.Steps) == 0 {
		return 0
	}

	completed := 0
	for _, step := range task.Plan.Steps {
		if step.Status == StepStatusCompleted || step.Status == StepStatusSkipped {
			completed++
		}
	}

	return (completed * 100) / len(task.Plan.Steps)
}

// emitEvent sends an event to the dispatcher
func (m *Manager) emitEvent(event TaskEvent) {
	if m.dispatcher != nil {
		m.dispatcher.Dispatch(event)
	}
}

// GetTasksForAgent returns all tasks for an agent
func (m *Manager) GetTasksForAgent(agentID string) []*Task {
	m.mu.RLock()
	defer m.mu.RUnlock()

	taskIDs, ok := m.byAgent[agentID]
	if !ok {
		return nil
	}

	tasks := make([]*Task, 0, len(taskIDs))
	for _, id := range taskIDs {
		if task, ok := m.tasks[id]; ok {
			tasks = append(tasks, task)
		}
	}
	return tasks
}

// CancelAllForAgent cancels all tasks for an agent
func (m *Manager) CancelAllForAgent(agentID string, reason string) {
	m.mu.Lock()
	taskIDs := m.byAgent[agentID]
	m.mu.Unlock()

	for _, id := range taskIDs {
		_ = m.Cancel(id, reason)
	}
}
