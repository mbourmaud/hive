package task

import "time"

// Plan represents a work plan created by the Queen
type Plan struct {
	ID            string            `json:"id"`
	Title         string            `json:"title"`
	Description   string            `json:"description,omitempty"`
	Context       string            `json:"context,omitempty"`
	Steps         []Step            `json:"steps"`
	RequiredPorts []PortRequirement `json:"required_ports,omitempty"`
	OnBlocker     string            `json:"on_blocker,omitempty"`
	OnAmbiguity   string            `json:"on_ambiguity,omitempty"`
	OnComplete    string            `json:"on_complete,omitempty"`
	CreatedAt     time.Time         `json:"created_at"`
	CreatedBy     string            `json:"created_by"`
}

// PortRequirement defines a port needed for the plan
type PortRequirement struct {
	Port    int      `json:"port"`
	Service string   `json:"service"`
	Phases  []string `json:"phases,omitempty"`
}

// Step represents a single step in a plan
type Step struct {
	ID          int           `json:"id"`
	Action      string        `json:"action"`
	Description string        `json:"description,omitempty"`
	DoD         []string      `json:"dod"`
	Autonomy    AutonomyLevel `json:"autonomy"`
	Status      StepStatus    `json:"status"`
	Result      string        `json:"result,omitempty"`
	Error       string        `json:"error,omitempty"`
	StartedAt   *time.Time    `json:"started_at,omitempty"`
	CompletedAt *time.Time    `json:"completed_at,omitempty"`
}

// AutonomyLevel determines when an agent should consult the Queen
type AutonomyLevel string

const (
	// AutonomyFull - Agent does without asking, validates DoD themselves
	AutonomyFull AutonomyLevel = "full"
	// AutonomyAskIfUnclear - Agent does, but asks Queen if unclear
	AutonomyAskIfUnclear AutonomyLevel = "ask_if_unclear"
	// AutonomyValidateBeforeNext - Agent does, then asks for validation
	AutonomyValidateBeforeNext AutonomyLevel = "validate_before_next"
	// AutonomyNotifyWhenDone - Agent does and notifies when done
	AutonomyNotifyWhenDone AutonomyLevel = "notify_when_done"
)

// StepStatus represents the current state of a step
type StepStatus string

const (
	StepStatusPending    StepStatus = "pending"
	StepStatusInProgress StepStatus = "in_progress"
	StepStatusWaiting    StepStatus = "waiting"
	StepStatusBlocked    StepStatus = "blocked"
	StepStatusCompleted  StepStatus = "completed"
	StepStatusFailed     StepStatus = "failed"
	StepStatusSkipped    StepStatus = "skipped"
)

// Task represents a task assigned to an agent
type Task struct {
	ID          string     `json:"id"`
	AgentID     string     `json:"agent_id"`
	AgentName   string     `json:"agent_name,omitempty"`
	Plan        Plan       `json:"plan"`
	Status      TaskStatus `json:"status"`
	CurrentStep int        `json:"current_step"`
	Result      string     `json:"result,omitempty"`
	Error       string     `json:"error,omitempty"`
	Artifacts   []Artifact `json:"artifacts,omitempty"`
	CreatedAt   time.Time  `json:"created_at"`
	StartedAt   *time.Time `json:"started_at,omitempty"`
	CompletedAt *time.Time `json:"completed_at,omitempty"`
}

// TaskStatus represents the current state of a task
type TaskStatus string

const (
	TaskStatusPending    TaskStatus = "pending"
	TaskStatusAssigned   TaskStatus = "assigned"
	TaskStatusInProgress TaskStatus = "in_progress"
	TaskStatusWaiting    TaskStatus = "waiting"
	TaskStatusCompleted  TaskStatus = "completed"
	TaskStatusFailed     TaskStatus = "failed"
	TaskStatusCancelled  TaskStatus = "cancelled"
)

// Artifact represents a produced output from a task
type Artifact struct {
	Type string `json:"type"` // "mr", "file", "url"
	Name string `json:"name"`
	URL  string `json:"url,omitempty"`
	Path string `json:"path,omitempty"`
}

// TaskEvent represents a task-related event
type TaskEvent struct {
	Type      string    `json:"type"` // "created", "started", "progress", "completed", "failed"
	Task      *Task     `json:"task,omitempty"`
	Step      *Step     `json:"step,omitempty"`
	Progress  int       `json:"progress,omitempty"` // 0-100
	Message   string    `json:"message,omitempty"`
	Timestamp time.Time `json:"timestamp"`
}

// CreateTaskRequest is the request to create a new task
type CreateTaskRequest struct {
	AgentID       string              `json:"agent_id"`
	AgentName     string              `json:"agent_name,omitempty"`
	Title         string              `json:"title"`
	Description   string              `json:"description,omitempty"`
	Context       string              `json:"context,omitempty"`
	Steps         []CreateStepRequest `json:"steps"`
	RequiredPorts []PortRequirement   `json:"required_ports,omitempty"`
	OnBlocker     string              `json:"on_blocker,omitempty"`
	OnAmbiguity   string              `json:"on_ambiguity,omitempty"`
	OnComplete    string              `json:"on_complete,omitempty"`
}

// CreateStepRequest is a step definition for creating a task
type CreateStepRequest struct {
	Action      string        `json:"action"`
	Description string        `json:"description,omitempty"`
	DoD         []string      `json:"dod"`
	Autonomy    AutonomyLevel `json:"autonomy"`
}

// UpdateStepRequest is the request to update a step's status
type UpdateStepRequest struct {
	Status StepStatus `json:"status"`
	Result string     `json:"result,omitempty"`
	Error  string     `json:"error,omitempty"`
}

// CompleteTaskRequest is the request to mark a task as complete
type CompleteTaskRequest struct {
	Result    string     `json:"result,omitempty"`
	Artifacts []Artifact `json:"artifacts,omitempty"`
}

// FailTaskRequest is the request to mark a task as failed
type FailTaskRequest struct {
	Error string `json:"error"`
}
