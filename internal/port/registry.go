package port

import (
	"context"
	"fmt"
	"sync"
	"syscall"
	"time"

	"github.com/mbourmaud/hive/internal/event"
)

// EventHandler is called when port events occur
type EventHandler func(event PortEvent)

// Registry manages port allocation and coordination between agents
type Registry struct {
	mu         sync.RWMutex
	leases     map[int]*PortLease
	waiters    map[int][]*waiterEntry
	dispatcher *event.Dispatcher[PortEvent]
}

type waiterEntry struct {
	waiter   PortWaiter
	notifyCh chan struct{}
}

// NewRegistry creates a new port registry
func NewRegistry(handler EventHandler) *Registry {
	r := &Registry{
		leases:  make(map[int]*PortLease),
		waiters: make(map[int][]*waiterEntry),
	}
	if handler != nil {
		r.dispatcher = event.NewDispatcher(handler, 4, 100)
		r.dispatcher.Start()
	}
	return r
}

// Close shuts down the registry and its event dispatcher.
func (r *Registry) Close() {
	if r.dispatcher != nil {
		r.dispatcher.Stop()
	}
}

// Acquire attempts to acquire a port for an agent
// If wait is true and the port is busy, it will block until available or timeout
func (r *Registry) Acquire(ctx context.Context, req AcquireRequest) (*AcquireResponse, error) {
	// Try to acquire immediately or set up waiting
	response, entry, shouldWait := r.tryAcquire(req)
	if !shouldWait {
		return response, nil
	}

	// Wait for port to become available
	var timeoutCh <-chan time.Time
	if req.Timeout > 0 {
		timeoutCh = time.After(time.Duration(req.Timeout) * time.Second)
	}

	select {
	case <-entry.notifyCh:
		// Port released, try to acquire again
		return r.Acquire(ctx, AcquireRequest{
			AgentID:   req.AgentID,
			AgentName: req.AgentName,
			Port:      req.Port,
			Service:   req.Service,
			Wait:      false, // Don't wait again, we were notified
		})

	case <-timeoutCh:
		r.removeWaiter(req.Port, entry)
		r.emitEvent(PortEvent{
			Type:      "timeout",
			Port:      req.Port,
			AgentID:   req.AgentID,
			AgentName: req.AgentName,
			Timestamp: time.Now(),
		})
		return &AcquireResponse{
			Status:  "timeout",
			Port:    req.Port,
			Message: fmt.Sprintf("timeout waiting for port %d", req.Port),
		}, nil

	case <-ctx.Done():
		r.removeWaiter(req.Port, entry)
		return nil, ctx.Err()
	}
}

// tryAcquire attempts to acquire the port immediately.
// Returns (response, nil, false) if acquisition succeeded or failed without waiting.
// Returns (nil, entry, true) if the caller should wait on entry.notifyCh.
func (r *Registry) tryAcquire(req AcquireRequest) (*AcquireResponse, *waiterEntry, bool) {
	r.mu.Lock()
	defer r.mu.Unlock()

	// Check if port is already leased
	if existing, ok := r.leases[req.Port]; ok {
		// Port is busy
		if !req.Wait {
			return &AcquireResponse{
				Status:  "busy",
				Port:    req.Port,
				HeldBy:  existing,
				Message: fmt.Sprintf("port %d is held by %s", req.Port, existing.AgentName),
			}, nil, false
		}

		// Create waiter entry
		entry := &waiterEntry{
			waiter: PortWaiter{
				AgentID:      req.AgentID,
				AgentName:    req.AgentName,
				Port:         req.Port,
				Service:      req.Service,
				WaitingSince: time.Now(),
				Timeout:      req.Timeout,
			},
			notifyCh: make(chan struct{}, 1),
		}

		r.waiters[req.Port] = append(r.waiters[req.Port], entry)

		// Emit waiting event
		r.emitEvent(PortEvent{
			Type:      "waiting",
			Port:      req.Port,
			AgentID:   req.AgentID,
			AgentName: req.AgentName,
			Service:   req.Service,
			HeldBy:    existing,
			Timestamp: time.Now(),
		})

		return nil, entry, true
	}

	// Port is free, create lease
	lease := &PortLease{
		Port:      req.Port,
		AgentID:   req.AgentID,
		AgentName: req.AgentName,
		Service:   req.Service,
		LeasedAt:  time.Now(),
	}
	r.leases[req.Port] = lease

	r.emitEvent(PortEvent{
		Type:      "acquired",
		Port:      req.Port,
		AgentID:   req.AgentID,
		AgentName: req.AgentName,
		Service:   req.Service,
		Timestamp: time.Now(),
	})

	return &AcquireResponse{
		Status:  "acquired",
		Port:    req.Port,
		Lease:   lease,
		Message: fmt.Sprintf("port %d acquired", req.Port),
	}, nil, false
}

// Release releases a port and notifies the next waiter
func (r *Registry) Release(req ReleaseRequest) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	lease, ok := r.leases[req.Port]
	if !ok {
		return fmt.Errorf("port %d is not leased", req.Port)
	}

	// Verify the agent owns the lease
	if lease.AgentID != req.AgentID {
		return fmt.Errorf("port %d is leased by %s, not %s", req.Port, lease.AgentID, req.AgentID)
	}

	delete(r.leases, req.Port)

	r.emitEvent(PortEvent{
		Type:      "released",
		Port:      req.Port,
		AgentID:   lease.AgentID,
		AgentName: lease.AgentName,
		Service:   lease.Service,
		Timestamp: time.Now(),
	})

	// Notify the first waiter
	r.notifyNextWaiter(req.Port)

	return nil
}

// ForceRelease forcefully releases a port, optionally killing the process
func (r *Registry) ForceRelease(req ForceReleaseRequest) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	lease, ok := r.leases[req.Port]
	if !ok {
		return fmt.Errorf("port %d is not leased", req.Port)
	}

	// Kill the process if we have a PID
	if lease.ProcessID > 0 {
		_ = syscall.Kill(lease.ProcessID, syscall.SIGTERM)
	}

	delete(r.leases, req.Port)

	r.emitEvent(PortEvent{
		Type:      "released",
		Port:      req.Port,
		AgentID:   lease.AgentID,
		AgentName: lease.AgentName,
		Service:   lease.Service,
		Timestamp: time.Now(),
	})

	// Notify the first waiter
	r.notifyNextWaiter(req.Port)

	return nil
}

// ReleaseAllForAgent releases all ports held by an agent
func (r *Registry) ReleaseAllForAgent(agentID string) []int {
	r.mu.Lock()
	defer r.mu.Unlock()

	released := make([]int, 0)
	for port, lease := range r.leases {
		if lease.AgentID == agentID {
			delete(r.leases, port)
			released = append(released, port)

			r.emitEvent(PortEvent{
				Type:      "released",
				Port:      port,
				AgentID:   lease.AgentID,
				AgentName: lease.AgentName,
				Service:   lease.Service,
				Timestamp: time.Now(),
			})

			r.notifyNextWaiter(port)
		}
	}

	// Also remove from waiters
	for port, entries := range r.waiters {
		filtered := make([]*waiterEntry, 0)
		for _, entry := range entries {
			if entry.waiter.AgentID != agentID {
				filtered = append(filtered, entry)
			}
		}
		if len(filtered) > 0 {
			r.waiters[port] = filtered
		} else {
			delete(r.waiters, port)
		}
	}

	return released
}

// GetStatus returns the status of a specific port
func (r *Registry) GetStatus(port int) PortStatus {
	r.mu.RLock()
	defer r.mu.RUnlock()

	status := PortStatus{
		Port:   port,
		Status: "free",
	}

	if lease, ok := r.leases[port]; ok {
		status.Status = "leased"
		status.Lease = lease
	}

	if entries, ok := r.waiters[port]; ok && len(entries) > 0 {
		if status.Status == "free" {
			status.Status = "waiting"
		}
		status.Waiters = make([]PortWaiter, len(entries))
		for i, entry := range entries {
			status.Waiters[i] = entry.waiter
		}
	}

	return status
}

// ListLeases returns all current leases
func (r *Registry) ListLeases() []PortLease {
	r.mu.RLock()
	defer r.mu.RUnlock()

	leases := make([]PortLease, 0, len(r.leases))
	for _, lease := range r.leases {
		leases = append(leases, *lease)
	}
	return leases
}

// ListWaiters returns all current waiters
func (r *Registry) ListWaiters() []PortWaiter {
	r.mu.RLock()
	defer r.mu.RUnlock()

	waiters := make([]PortWaiter, 0)
	for _, entries := range r.waiters {
		for _, entry := range entries {
			waiters = append(waiters, entry.waiter)
		}
	}
	return waiters
}

// GetLeasesForAgent returns all ports leased by an agent
func (r *Registry) GetLeasesForAgent(agentID string) []PortLease {
	r.mu.RLock()
	defer r.mu.RUnlock()

	leases := make([]PortLease, 0)
	for _, lease := range r.leases {
		if lease.AgentID == agentID {
			leases = append(leases, *lease)
		}
	}
	return leases
}

// SetProcessID updates the process ID for a leased port
func (r *Registry) SetProcessID(port, pid int) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	lease, ok := r.leases[port]
	if !ok {
		return fmt.Errorf("port %d is not leased", port)
	}

	lease.ProcessID = pid
	return nil
}

// removeWaiter removes a waiter entry from the list
func (r *Registry) removeWaiter(port int, entry *waiterEntry) {
	r.mu.Lock()
	defer r.mu.Unlock()

	entries, ok := r.waiters[port]
	if !ok {
		return
	}

	filtered := make([]*waiterEntry, 0, len(entries))
	for _, e := range entries {
		if e != entry {
			filtered = append(filtered, e)
		}
	}

	if len(filtered) > 0 {
		r.waiters[port] = filtered
	} else {
		delete(r.waiters, port)
	}
}

// notifyNextWaiter notifies the next waiter in line
func (r *Registry) notifyNextWaiter(port int) {
	entries, ok := r.waiters[port]
	if !ok || len(entries) == 0 {
		return
	}

	// Notify the first waiter
	entry := entries[0]
	select {
	case entry.notifyCh <- struct{}{}:
	default:
	}

	// Remove from waiters
	if len(entries) > 1 {
		r.waiters[port] = entries[1:]
	} else {
		delete(r.waiters, port)
	}
}

// emitEvent sends an event to the dispatcher
func (r *Registry) emitEvent(event PortEvent) {
	if r.dispatcher != nil {
		r.dispatcher.Dispatch(event)
	}
}
