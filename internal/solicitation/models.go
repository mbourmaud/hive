package solicitation

import "time"

// Solicitation represents a request from an agent to the Queen
type Solicitation struct {
	ID          string            `json:"id"`
	AgentID     string            `json:"agent_id"`
	AgentName   string            `json:"agent_name"`
	TaskID      string            `json:"task_id,omitempty"`
	StepID      int               `json:"step_id,omitempty"`
	Type        Type              `json:"type"`
	Urgency     Urgency           `json:"urgency"`
	Message     string            `json:"message"`
	Context     string            `json:"context,omitempty"`
	Options     []string          `json:"options,omitempty"`
	Metadata    map[string]string `json:"metadata,omitempty"`
	Status      Status            `json:"status"`
	Response    string            `json:"response,omitempty"`
	CreatedAt   time.Time         `json:"created_at"`
	RespondedAt *time.Time        `json:"responded_at,omitempty"`
}

// Type represents the type of solicitation
type Type string

const (
	// TypeBlocker - Technical blocker (error, missing dependency)
	TypeBlocker Type = "blocker"
	// TypeAmbiguity - Needs clarification (unclear specs)
	TypeAmbiguity Type = "ambiguity"
	// TypeDecision - Choice to be made (multiple options)
	TypeDecision Type = "decision"
	// TypeValidation - Needs validation before continuing
	TypeValidation Type = "validation"
	// TypeInfo - Needs information
	TypeInfo Type = "info"
	// TypeCompletion - Task completed
	TypeCompletion Type = "completion"
	// TypeProgress - Progress update
	TypeProgress Type = "progress"
	// TypeResourceConflict - Resource conflict (port busy, etc.)
	TypeResourceConflict Type = "resource_conflict"
)

// Urgency represents how urgent a solicitation is
type Urgency string

const (
	UrgencyLow      Urgency = "low"
	UrgencyMedium   Urgency = "medium"
	UrgencyHigh     Urgency = "high"
	UrgencyCritical Urgency = "critical"
)

// Status represents the status of a solicitation
type Status string

const (
	StatusPending   Status = "pending"
	StatusResponded Status = "responded"
	StatusDismissed Status = "dismissed"
	StatusExpired   Status = "expired"
)

// Event represents a solicitation-related event
type Event struct {
	Type         string        `json:"type"` // "new", "responded", "dismissed", "expired"
	Solicitation *Solicitation `json:"solicitation"`
	Timestamp    time.Time     `json:"timestamp"`
}

// CreateRequest is the request to create a solicitation
type CreateRequest struct {
	AgentID   string            `json:"agent_id"`
	AgentName string            `json:"agent_name,omitempty"`
	TaskID    string            `json:"task_id,omitempty"`
	StepID    int               `json:"step_id,omitempty"`
	Type      Type              `json:"type"`
	Urgency   Urgency           `json:"urgency"`
	Message   string            `json:"message"`
	Context   string            `json:"context,omitempty"`
	Options   []string          `json:"options,omitempty"`
	Metadata  map[string]string `json:"metadata,omitempty"`
}

// RespondRequest is the request to respond to a solicitation
type RespondRequest struct {
	Response string `json:"response"`
}

// DismissRequest is the request to dismiss a solicitation
type DismissRequest struct {
	Reason string `json:"reason,omitempty"`
}

// ListFilter contains filters for listing solicitations
type ListFilter struct {
	AgentID string
	Type    Type
	Urgency Urgency
	Status  Status
}
