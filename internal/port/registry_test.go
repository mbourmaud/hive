package port

import (
	"context"
	"sync"
	"testing"
	"time"
)

func TestRegistry_Acquire_FreePort(t *testing.T) {
	events := make([]PortEvent, 0)
	var mu sync.Mutex

	registry := NewRegistry(func(e PortEvent) {
		mu.Lock()
		events = append(events, e)
		mu.Unlock()
	})

	resp, err := registry.Acquire(context.Background(), AcquireRequest{
		AgentID:   "agent-1",
		AgentName: "Agent One",
		Port:      3000,
		Service:   "frontend",
	})

	if err != nil {
		t.Fatalf("Acquire() error = %v", err)
	}

	if resp.Status != "acquired" {
		t.Errorf("Acquire() status = %s, want acquired", resp.Status)
	}

	if resp.Lease == nil {
		t.Fatal("Acquire() lease is nil")
	}

	if resp.Lease.Port != 3000 {
		t.Errorf("Acquire() lease.Port = %d, want 3000", resp.Lease.Port)
	}

	// Check event was emitted
	time.Sleep(10 * time.Millisecond)
	mu.Lock()
	defer mu.Unlock()
	if len(events) != 1 {
		t.Errorf("Expected 1 event, got %d", len(events))
	}
	if events[0].Type != "acquired" {
		t.Errorf("Event type = %s, want acquired", events[0].Type)
	}
}

func TestRegistry_Acquire_BusyPort_NoWait(t *testing.T) {
	registry := NewRegistry(nil)

	// First agent acquires
	_, err := registry.Acquire(context.Background(), AcquireRequest{
		AgentID:   "agent-1",
		AgentName: "Agent One",
		Port:      3000,
		Service:   "frontend",
	})
	if err != nil {
		t.Fatalf("First Acquire() error = %v", err)
	}

	// Second agent tries without waiting
	resp, err := registry.Acquire(context.Background(), AcquireRequest{
		AgentID:   "agent-2",
		AgentName: "Agent Two",
		Port:      3000,
		Service:   "frontend",
		Wait:      false,
	})

	if err != nil {
		t.Fatalf("Second Acquire() error = %v", err)
	}

	if resp.Status != "busy" {
		t.Errorf("Acquire() status = %s, want busy", resp.Status)
	}

	if resp.HeldBy == nil {
		t.Fatal("Acquire() held_by is nil")
	}

	if resp.HeldBy.AgentID != "agent-1" {
		t.Errorf("Acquire() held_by.AgentID = %s, want agent-1", resp.HeldBy.AgentID)
	}
}

func TestRegistry_Acquire_BusyPort_WithWait(t *testing.T) {
	registry := NewRegistry(nil)

	// First agent acquires
	_, err := registry.Acquire(context.Background(), AcquireRequest{
		AgentID:   "agent-1",
		AgentName: "Agent One",
		Port:      3000,
		Service:   "frontend",
	})
	if err != nil {
		t.Fatalf("First Acquire() error = %v", err)
	}

	// Second agent waits in background
	var wg sync.WaitGroup
	var resp2 *AcquireResponse
	var err2 error

	wg.Add(1)
	go func() {
		defer wg.Done()
		resp2, err2 = registry.Acquire(context.Background(), AcquireRequest{
			AgentID:   "agent-2",
			AgentName: "Agent Two",
			Port:      3000,
			Service:   "frontend",
			Wait:      true,
		})
	}()

	// Wait a bit then release
	time.Sleep(50 * time.Millisecond)
	err = registry.Release(ReleaseRequest{
		AgentID: "agent-1",
		Port:    3000,
	})
	if err != nil {
		t.Fatalf("Release() error = %v", err)
	}

	// Wait for second acquire to complete
	wg.Wait()

	if err2 != nil {
		t.Fatalf("Second Acquire() error = %v", err2)
	}

	if resp2.Status != "acquired" {
		t.Errorf("Second Acquire() status = %s, want acquired", resp2.Status)
	}

	if resp2.Lease.AgentID != "agent-2" {
		t.Errorf("Second Acquire() lease.AgentID = %s, want agent-2", resp2.Lease.AgentID)
	}
}

func TestRegistry_Acquire_Timeout(t *testing.T) {
	registry := NewRegistry(nil)

	// First agent acquires
	_, err := registry.Acquire(context.Background(), AcquireRequest{
		AgentID:   "agent-1",
		AgentName: "Agent One",
		Port:      3000,
		Service:   "frontend",
	})
	if err != nil {
		t.Fatalf("First Acquire() error = %v", err)
	}

	// Second agent waits with timeout
	start := time.Now()
	resp, err := registry.Acquire(context.Background(), AcquireRequest{
		AgentID:   "agent-2",
		AgentName: "Agent Two",
		Port:      3000,
		Service:   "frontend",
		Wait:      true,
		Timeout:   1, // 1 second
	})
	elapsed := time.Since(start)

	if err != nil {
		t.Fatalf("Acquire() error = %v", err)
	}

	if resp.Status != "timeout" {
		t.Errorf("Acquire() status = %s, want timeout", resp.Status)
	}

	if elapsed < 900*time.Millisecond {
		t.Errorf("Acquire() returned too quickly: %v", elapsed)
	}
}

func TestRegistry_Release(t *testing.T) {
	registry := NewRegistry(nil)

	// Acquire
	_, err := registry.Acquire(context.Background(), AcquireRequest{
		AgentID:   "agent-1",
		AgentName: "Agent One",
		Port:      3000,
		Service:   "frontend",
	})
	if err != nil {
		t.Fatalf("Acquire() error = %v", err)
	}

	// Release
	err = registry.Release(ReleaseRequest{
		AgentID: "agent-1",
		Port:    3000,
	})
	if err != nil {
		t.Fatalf("Release() error = %v", err)
	}

	// Port should be free now
	status := registry.GetStatus(3000)
	if status.Status != "free" {
		t.Errorf("GetStatus() status = %s, want free", status.Status)
	}
}

func TestRegistry_Release_WrongAgent(t *testing.T) {
	registry := NewRegistry(nil)

	// Acquire
	_, err := registry.Acquire(context.Background(), AcquireRequest{
		AgentID:   "agent-1",
		AgentName: "Agent One",
		Port:      3000,
		Service:   "frontend",
	})
	if err != nil {
		t.Fatalf("Acquire() error = %v", err)
	}

	// Try to release with wrong agent
	err = registry.Release(ReleaseRequest{
		AgentID: "agent-2",
		Port:    3000,
	})
	if err == nil {
		t.Fatal("Release() should have failed")
	}
}

func TestRegistry_ForceRelease(t *testing.T) {
	registry := NewRegistry(nil)

	// Acquire
	_, err := registry.Acquire(context.Background(), AcquireRequest{
		AgentID:   "agent-1",
		AgentName: "Agent One",
		Port:      3000,
		Service:   "frontend",
	})
	if err != nil {
		t.Fatalf("Acquire() error = %v", err)
	}

	// Force release
	err = registry.ForceRelease(ForceReleaseRequest{
		Port:   3000,
		Reason: "testing",
	})
	if err != nil {
		t.Fatalf("ForceRelease() error = %v", err)
	}

	// Port should be free
	status := registry.GetStatus(3000)
	if status.Status != "free" {
		t.Errorf("GetStatus() status = %s, want free", status.Status)
	}
}

func TestRegistry_ReleaseAllForAgent(t *testing.T) {
	registry := NewRegistry(nil)

	// Acquire multiple ports
	for _, port := range []int{3000, 3001, 3002} {
		_, err := registry.Acquire(context.Background(), AcquireRequest{
			AgentID:   "agent-1",
			AgentName: "Agent One",
			Port:      port,
			Service:   "test",
		})
		if err != nil {
			t.Fatalf("Acquire(%d) error = %v", port, err)
		}
	}

	// Release all
	released := registry.ReleaseAllForAgent("agent-1")

	if len(released) != 3 {
		t.Errorf("ReleaseAllForAgent() released %d ports, want 3", len(released))
	}

	// All ports should be free
	for _, port := range []int{3000, 3001, 3002} {
		status := registry.GetStatus(port)
		if status.Status != "free" {
			t.Errorf("Port %d status = %s, want free", port, status.Status)
		}
	}
}

func TestRegistry_ListLeases(t *testing.T) {
	registry := NewRegistry(nil)

	// Acquire some ports
	for _, port := range []int{3000, 3001} {
		_, err := registry.Acquire(context.Background(), AcquireRequest{
			AgentID:   "agent-1",
			AgentName: "Agent One",
			Port:      port,
			Service:   "test",
		})
		if err != nil {
			t.Fatalf("Acquire(%d) error = %v", port, err)
		}
	}

	leases := registry.ListLeases()
	if len(leases) != 2 {
		t.Errorf("ListLeases() returned %d leases, want 2", len(leases))
	}
}

func TestRegistry_GetLeasesForAgent(t *testing.T) {
	registry := NewRegistry(nil)

	// Agent 1 acquires ports
	for _, port := range []int{3000, 3001} {
		_, _ = registry.Acquire(context.Background(), AcquireRequest{
			AgentID:   "agent-1",
			AgentName: "Agent One",
			Port:      port,
			Service:   "test",
		})
	}

	// Agent 2 acquires a port
	_, _ = registry.Acquire(context.Background(), AcquireRequest{
		AgentID:   "agent-2",
		AgentName: "Agent Two",
		Port:      3002,
		Service:   "test",
	})

	leases := registry.GetLeasesForAgent("agent-1")
	if len(leases) != 2 {
		t.Errorf("GetLeasesForAgent() returned %d leases, want 2", len(leases))
	}
}

func TestRegistry_SetProcessID(t *testing.T) {
	registry := NewRegistry(nil)

	_, err := registry.Acquire(context.Background(), AcquireRequest{
		AgentID:   "agent-1",
		AgentName: "Agent One",
		Port:      3000,
		Service:   "frontend",
	})
	if err != nil {
		t.Fatalf("Acquire() error = %v", err)
	}

	err = registry.SetProcessID(3000, 12345)
	if err != nil {
		t.Fatalf("SetProcessID() error = %v", err)
	}

	status := registry.GetStatus(3000)
	if status.Lease.ProcessID != 12345 {
		t.Errorf("ProcessID = %d, want 12345", status.Lease.ProcessID)
	}
}
