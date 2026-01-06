package solicitation

import (
	"fmt"
	"sort"
	"sync"
	"time"

	"github.com/google/uuid"
	"github.com/mbourmaud/hive/internal/event"
)

// EventHandler is called when solicitation events occur
type EventHandler func(event Event)

// Manager manages solicitations from agents
type Manager struct {
	mu              sync.RWMutex
	solicitations   map[string]*Solicitation
	byAgent         map[string][]string // agentID -> solicitationIDs
	pending         []string            // ordered by urgency then time
	dispatcher      *event.Dispatcher[Event]
	responseWaiters map[string]chan string // solicitationID -> response channel
}

// NewManager creates a new solicitation manager
func NewManager(handler EventHandler) *Manager {
	m := &Manager{
		solicitations:   make(map[string]*Solicitation),
		byAgent:         make(map[string][]string),
		pending:         make([]string, 0),
		responseWaiters: make(map[string]chan string),
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

// Create creates a new solicitation
func (m *Manager) Create(req CreateRequest) (*Solicitation, error) {
	if req.AgentID == "" {
		return nil, fmt.Errorf("agent_id is required")
	}
	if req.Message == "" {
		return nil, fmt.Errorf("message is required")
	}
	if req.Type == "" {
		return nil, fmt.Errorf("type is required")
	}

	urgency := req.Urgency
	if urgency == "" {
		urgency = UrgencyMedium
	}

	sol := &Solicitation{
		ID:        uuid.New().String(),
		AgentID:   req.AgentID,
		AgentName: req.AgentName,
		TaskID:    req.TaskID,
		StepID:    req.StepID,
		Type:      req.Type,
		Urgency:   urgency,
		Message:   req.Message,
		Context:   req.Context,
		Options:   req.Options,
		Metadata:  req.Metadata,
		Status:    StatusPending,
		CreatedAt: time.Now(),
	}

	m.mu.Lock()
	m.solicitations[sol.ID] = sol
	m.byAgent[req.AgentID] = append(m.byAgent[req.AgentID], sol.ID)
	m.addToPending(sol)
	m.mu.Unlock()

	m.emitEvent(Event{
		Type:         "new",
		Solicitation: sol,
		Timestamp:    time.Now(),
	})

	return sol, nil
}

// CreateAndWait creates a solicitation and waits for a response
func (m *Manager) CreateAndWait(req CreateRequest, timeout time.Duration) (*Solicitation, string, error) {
	sol, err := m.Create(req)
	if err != nil {
		return nil, "", err
	}

	// Create response channel
	responseCh := make(chan string, 1)
	m.mu.Lock()
	m.responseWaiters[sol.ID] = responseCh
	m.mu.Unlock()

	defer func() {
		m.mu.Lock()
		delete(m.responseWaiters, sol.ID)
		m.mu.Unlock()
	}()

	// Wait for response
	if timeout > 0 {
		select {
		case response := <-responseCh:
			updated, _ := m.Get(sol.ID)
			return updated, response, nil
		case <-time.After(timeout):
			_ = m.Expire(sol.ID)
			return sol, "", fmt.Errorf("timeout waiting for response")
		}
	} else {
		response := <-responseCh
		updated, _ := m.Get(sol.ID)
		return updated, response, nil
	}
}

// Get returns a solicitation by ID
func (m *Manager) Get(id string) (*Solicitation, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	sol, ok := m.solicitations[id]
	if !ok {
		return nil, fmt.Errorf("solicitation %s not found", id)
	}
	return sol, nil
}

// List returns solicitations matching the filter
func (m *Manager) List(filter ListFilter) []*Solicitation {
	m.mu.RLock()
	defer m.mu.RUnlock()

	result := make([]*Solicitation, 0)
	for _, sol := range m.solicitations {
		if filter.AgentID != "" && sol.AgentID != filter.AgentID {
			continue
		}
		if filter.Type != "" && sol.Type != filter.Type {
			continue
		}
		if filter.Urgency != "" && sol.Urgency != filter.Urgency {
			continue
		}
		if filter.Status != "" && sol.Status != filter.Status {
			continue
		}
		result = append(result, sol)
	}

	// Sort by urgency (critical first) then by creation time
	sort.Slice(result, func(i, j int) bool {
		ui := urgencyOrder(result[i].Urgency)
		uj := urgencyOrder(result[j].Urgency)
		if ui != uj {
			return ui < uj
		}
		return result[i].CreatedAt.Before(result[j].CreatedAt)
	})

	return result
}

// ListPending returns all pending solicitations ordered by priority
func (m *Manager) ListPending() []*Solicitation {
	return m.List(ListFilter{Status: StatusPending})
}

// Respond responds to a solicitation
func (m *Manager) Respond(id string, req RespondRequest) error {
	m.mu.Lock()

	sol, ok := m.solicitations[id]
	if !ok {
		m.mu.Unlock()
		return fmt.Errorf("solicitation %s not found", id)
	}

	if sol.Status != StatusPending {
		m.mu.Unlock()
		return fmt.Errorf("solicitation %s is not pending", id)
	}

	now := time.Now()
	sol.Status = StatusResponded
	sol.Response = req.Response
	sol.RespondedAt = &now

	m.removeFromPending(id)

	// Notify waiter if any
	if ch, ok := m.responseWaiters[id]; ok {
		select {
		case ch <- req.Response:
		default:
		}
	}

	m.mu.Unlock()

	m.emitEvent(Event{
		Type:         "responded",
		Solicitation: sol,
		Timestamp:    now,
	})

	return nil
}

// Dismiss dismisses a solicitation
func (m *Manager) Dismiss(id string, req DismissRequest) error {
	m.mu.Lock()

	sol, ok := m.solicitations[id]
	if !ok {
		m.mu.Unlock()
		return fmt.Errorf("solicitation %s not found", id)
	}

	if sol.Status != StatusPending {
		m.mu.Unlock()
		return fmt.Errorf("solicitation %s is not pending", id)
	}

	now := time.Now()
	sol.Status = StatusDismissed
	sol.Response = req.Reason
	sol.RespondedAt = &now

	m.removeFromPending(id)

	// Notify waiter if any (with empty response)
	if ch, ok := m.responseWaiters[id]; ok {
		close(ch)
		delete(m.responseWaiters, id)
	}

	m.mu.Unlock()

	m.emitEvent(Event{
		Type:         "dismissed",
		Solicitation: sol,
		Timestamp:    now,
	})

	return nil
}

// Expire marks a solicitation as expired
func (m *Manager) Expire(id string) error {
	m.mu.Lock()

	sol, ok := m.solicitations[id]
	if !ok {
		m.mu.Unlock()
		return fmt.Errorf("solicitation %s not found", id)
	}

	if sol.Status != StatusPending {
		m.mu.Unlock()
		return nil // Already handled
	}

	now := time.Now()
	sol.Status = StatusExpired
	sol.RespondedAt = &now

	m.removeFromPending(id)

	m.mu.Unlock()

	m.emitEvent(Event{
		Type:         "expired",
		Solicitation: sol,
		Timestamp:    now,
	})

	return nil
}

// Delete removes a solicitation
func (m *Manager) Delete(id string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	sol, ok := m.solicitations[id]
	if !ok {
		return fmt.Errorf("solicitation %s not found", id)
	}

	// Remove from byAgent
	agentSols := m.byAgent[sol.AgentID]
	filtered := make([]string, 0, len(agentSols))
	for _, sid := range agentSols {
		if sid != id {
			filtered = append(filtered, sid)
		}
	}
	if len(filtered) > 0 {
		m.byAgent[sol.AgentID] = filtered
	} else {
		delete(m.byAgent, sol.AgentID)
	}

	m.removeFromPending(id)
	delete(m.solicitations, id)

	return nil
}

// GetByAgent returns all solicitations for an agent
func (m *Manager) GetByAgent(agentID string) []*Solicitation {
	m.mu.RLock()
	defer m.mu.RUnlock()

	ids, ok := m.byAgent[agentID]
	if !ok {
		return nil
	}

	result := make([]*Solicitation, 0, len(ids))
	for _, id := range ids {
		if sol, ok := m.solicitations[id]; ok {
			result = append(result, sol)
		}
	}
	return result
}

// GetPendingByAgent returns pending solicitations for an agent
func (m *Manager) GetPendingByAgent(agentID string) []*Solicitation {
	sols := m.GetByAgent(agentID)
	result := make([]*Solicitation, 0)
	for _, sol := range sols {
		if sol.Status == StatusPending {
			result = append(result, sol)
		}
	}
	return result
}

// DismissAllForAgent dismisses all pending solicitations for an agent
func (m *Manager) DismissAllForAgent(agentID string, reason string) {
	sols := m.GetPendingByAgent(agentID)
	for _, sol := range sols {
		_ = m.Dismiss(sol.ID, DismissRequest{Reason: reason})
	}
}

// Count returns counts of solicitations by status
func (m *Manager) Count() map[Status]int {
	m.mu.RLock()
	defer m.mu.RUnlock()

	counts := make(map[Status]int)
	for _, sol := range m.solicitations {
		counts[sol.Status]++
	}
	return counts
}

// addToPending adds a solicitation to the pending list in priority order
func (m *Manager) addToPending(sol *Solicitation) {
	// Find insertion point
	order := urgencyOrder(sol.Urgency)
	insertAt := len(m.pending)

	for i, id := range m.pending {
		existing := m.solicitations[id]
		if existing == nil {
			continue
		}
		existingOrder := urgencyOrder(existing.Urgency)
		if order < existingOrder {
			insertAt = i
			break
		}
		if order == existingOrder && sol.CreatedAt.Before(existing.CreatedAt) {
			insertAt = i
			break
		}
	}

	// Insert
	m.pending = append(m.pending[:insertAt], append([]string{sol.ID}, m.pending[insertAt:]...)...)
}

// removeFromPending removes a solicitation from the pending list
func (m *Manager) removeFromPending(id string) {
	filtered := make([]string, 0, len(m.pending))
	for _, sid := range m.pending {
		if sid != id {
			filtered = append(filtered, sid)
		}
	}
	m.pending = filtered
}

// emitEvent sends an event to the dispatcher
func (m *Manager) emitEvent(event Event) {
	if m.dispatcher != nil {
		m.dispatcher.Dispatch(event)
	}
}

// urgencyOrder returns the sort order for urgency (lower = higher priority)
func urgencyOrder(u Urgency) int {
	switch u {
	case UrgencyCritical:
		return 0
	case UrgencyHigh:
		return 1
	case UrgencyMedium:
		return 2
	case UrgencyLow:
		return 3
	default:
		return 4
	}
}
