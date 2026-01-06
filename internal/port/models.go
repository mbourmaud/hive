package port

import "time"

// PortLease represents a port currently in use by an agent
type PortLease struct {
	Port      int       `json:"port"`
	AgentID   string    `json:"agent_id"`
	AgentName string    `json:"agent_name"`
	Service   string    `json:"service"`
	ProcessID int       `json:"process_id,omitempty"`
	LeasedAt  time.Time `json:"leased_at"`
}

// PortWaiter represents an agent waiting for a port
type PortWaiter struct {
	AgentID      string    `json:"agent_id"`
	AgentName    string    `json:"agent_name"`
	Port         int       `json:"port"`
	Service      string    `json:"service"`
	WaitingSince time.Time `json:"waiting_since"`
	Timeout      int       `json:"timeout"` // Timeout in seconds, 0 = infinite
}

// PortStatus represents the complete status of a port
type PortStatus struct {
	Port    int          `json:"port"`
	Status  string       `json:"status"` // "free", "leased", "waiting"
	Lease   *PortLease   `json:"lease,omitempty"`
	Waiters []PortWaiter `json:"waiters,omitempty"`
}

// PortEvent represents a port-related event for notifications
type PortEvent struct {
	Type      string     `json:"type"` // "acquired", "released", "waiting", "timeout", "conflict"
	Port      int        `json:"port"`
	AgentID   string     `json:"agent_id"`
	AgentName string     `json:"agent_name"`
	Service   string     `json:"service,omitempty"`
	HeldBy    *PortLease `json:"held_by,omitempty"`
	Timestamp time.Time  `json:"timestamp"`
}

// AcquireRequest is the request to acquire a port
type AcquireRequest struct {
	AgentID   string `json:"agent_id"`
	AgentName string `json:"agent_name"`
	Port      int    `json:"port"`
	Service   string `json:"service"`
	Wait      bool   `json:"wait"`
	Timeout   int    `json:"timeout"` // Timeout in seconds
}

// AcquireResponse is the response to an acquire request
type AcquireResponse struct {
	Status  string     `json:"status"` // "acquired", "waiting", "busy", "timeout"
	Port    int        `json:"port"`
	Lease   *PortLease `json:"lease,omitempty"`
	HeldBy  *PortLease `json:"held_by,omitempty"`
	Message string     `json:"message,omitempty"`
}

// ReleaseRequest is the request to release a port
type ReleaseRequest struct {
	AgentID string `json:"agent_id"`
	Port    int    `json:"port"`
}

// ForceReleaseRequest is the request to force-release a port
type ForceReleaseRequest struct {
	Port   int    `json:"port"`
	Reason string `json:"reason,omitempty"`
}
