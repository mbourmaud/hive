package solicitation

import (
	"sync"
	"testing"
	"time"
)

func TestManager_Create(t *testing.T) {
	events := make([]Event, 0)
	var mu sync.Mutex

	mgr := NewManager(func(e Event) {
		mu.Lock()
		events = append(events, e)
		mu.Unlock()
	})

	sol, err := mgr.Create(CreateRequest{
		AgentID:   "agent-1",
		AgentName: "Agent One",
		Type:      TypeBlocker,
		Urgency:   UrgencyHigh,
		Message:   "I need help!",
		Context:   "Some context here",
		Options:   []string{"Option A", "Option B"},
	})

	if err != nil {
		t.Fatalf("Create() error = %v", err)
	}

	if sol.ID == "" {
		t.Error("Create() sol.ID is empty")
	}

	if sol.Status != StatusPending {
		t.Errorf("Create() status = %s, want pending", sol.Status)
	}

	if sol.Type != TypeBlocker {
		t.Errorf("Create() type = %s, want blocker", sol.Type)
	}

	// Check event was emitted
	time.Sleep(10 * time.Millisecond)
	mu.Lock()
	defer mu.Unlock()
	if len(events) != 1 {
		t.Errorf("Expected 1 event, got %d", len(events))
	}
	if events[0].Type != "new" {
		t.Errorf("Event type = %s, want new", events[0].Type)
	}
}

func TestManager_Create_Validation(t *testing.T) {
	mgr := NewManager(nil)

	tests := []struct {
		name    string
		req     CreateRequest
		wantErr bool
	}{
		{
			name:    "missing agent_id",
			req:     CreateRequest{Type: TypeBlocker, Message: "Help"},
			wantErr: true,
		},
		{
			name:    "missing type",
			req:     CreateRequest{AgentID: "a1", Message: "Help"},
			wantErr: true,
		},
		{
			name:    "missing message",
			req:     CreateRequest{AgentID: "a1", Type: TypeBlocker},
			wantErr: true,
		},
		{
			name:    "valid request",
			req:     CreateRequest{AgentID: "a1", Type: TypeBlocker, Message: "Help"},
			wantErr: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, err := mgr.Create(tt.req)
			if (err != nil) != tt.wantErr {
				t.Errorf("Create() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestManager_Respond(t *testing.T) {
	mgr := NewManager(nil)

	sol, _ := mgr.Create(CreateRequest{
		AgentID: "agent-1",
		Type:    TypeDecision,
		Message: "Which option?",
		Options: []string{"A", "B"},
	})

	err := mgr.Respond(sol.ID, RespondRequest{
		Response: "Go with A",
	})
	if err != nil {
		t.Fatalf("Respond() error = %v", err)
	}

	updated, _ := mgr.Get(sol.ID)
	if updated.Status != StatusResponded {
		t.Errorf("Respond() status = %s, want responded", updated.Status)
	}

	if updated.Response != "Go with A" {
		t.Errorf("Respond() response = %s, want 'Go with A'", updated.Response)
	}

	if updated.RespondedAt == nil {
		t.Error("Respond() responded_at is nil")
	}
}

func TestManager_Dismiss(t *testing.T) {
	mgr := NewManager(nil)

	sol, _ := mgr.Create(CreateRequest{
		AgentID: "agent-1",
		Type:    TypeProgress,
		Message: "Just an update",
	})

	err := mgr.Dismiss(sol.ID, DismissRequest{
		Reason: "Not important",
	})
	if err != nil {
		t.Fatalf("Dismiss() error = %v", err)
	}

	updated, _ := mgr.Get(sol.ID)
	if updated.Status != StatusDismissed {
		t.Errorf("Dismiss() status = %s, want dismissed", updated.Status)
	}
}

func TestManager_ListPending(t *testing.T) {
	mgr := NewManager(nil)

	// Create with different urgencies
	_, _ = mgr.Create(CreateRequest{
		AgentID: "agent-1",
		Type:    TypeInfo,
		Urgency: UrgencyLow,
		Message: "Low priority",
	})

	_, _ = mgr.Create(CreateRequest{
		AgentID: "agent-1",
		Type:    TypeBlocker,
		Urgency: UrgencyCritical,
		Message: "Critical!",
	})

	_, _ = mgr.Create(CreateRequest{
		AgentID: "agent-1",
		Type:    TypeDecision,
		Urgency: UrgencyMedium,
		Message: "Medium priority",
	})

	pending := mgr.ListPending()

	if len(pending) != 3 {
		t.Fatalf("ListPending() len = %d, want 3", len(pending))
	}

	// Critical should be first
	if pending[0].Urgency != UrgencyCritical {
		t.Errorf("ListPending()[0].Urgency = %s, want critical", pending[0].Urgency)
	}

	// Medium should be second
	if pending[1].Urgency != UrgencyMedium {
		t.Errorf("ListPending()[1].Urgency = %s, want medium", pending[1].Urgency)
	}

	// Low should be last
	if pending[2].Urgency != UrgencyLow {
		t.Errorf("ListPending()[2].Urgency = %s, want low", pending[2].Urgency)
	}
}

func TestManager_ListFilter(t *testing.T) {
	mgr := NewManager(nil)

	sol1, _ := mgr.Create(CreateRequest{
		AgentID: "agent-1",
		Type:    TypeBlocker,
		Urgency: UrgencyHigh,
		Message: "Blocker",
	})

	_, _ = mgr.Create(CreateRequest{
		AgentID: "agent-2",
		Type:    TypeDecision,
		Urgency: UrgencyMedium,
		Message: "Decision",
	})

	// Respond to first one
	_ = mgr.Respond(sol1.ID, RespondRequest{Response: "Fixed"})

	// Filter by agent
	agent1 := mgr.List(ListFilter{AgentID: "agent-1"})
	if len(agent1) != 1 {
		t.Errorf("List(agent-1) len = %d, want 1", len(agent1))
	}

	// Filter by type
	blockers := mgr.List(ListFilter{Type: TypeBlocker})
	if len(blockers) != 1 {
		t.Errorf("List(blocker) len = %d, want 1", len(blockers))
	}

	// Filter by status
	pending := mgr.List(ListFilter{Status: StatusPending})
	if len(pending) != 1 {
		t.Errorf("List(pending) len = %d, want 1", len(pending))
	}

	responded := mgr.List(ListFilter{Status: StatusResponded})
	if len(responded) != 1 {
		t.Errorf("List(responded) len = %d, want 1", len(responded))
	}
}

func TestManager_CreateAndWait(t *testing.T) {
	mgr := NewManager(nil)

	var sol *Solicitation
	var response string
	var err error

	done := make(chan struct{})

	go func() {
		sol, response, err = mgr.CreateAndWait(CreateRequest{
			AgentID: "agent-1",
			Type:    TypeDecision,
			Message: "Choose",
		}, 5*time.Second)
		close(done)
	}()

	// Wait for solicitation to be created
	time.Sleep(50 * time.Millisecond)

	// Get pending and respond
	pending := mgr.ListPending()
	if len(pending) != 1 {
		t.Fatalf("Expected 1 pending, got %d", len(pending))
	}

	_ = mgr.Respond(pending[0].ID, RespondRequest{Response: "Choice A"})

	<-done

	if err != nil {
		t.Fatalf("CreateAndWait() error = %v", err)
	}

	if response != "Choice A" {
		t.Errorf("CreateAndWait() response = %s, want 'Choice A'", response)
	}

	if sol.Status != StatusResponded {
		t.Errorf("CreateAndWait() status = %s, want responded", sol.Status)
	}
}

func TestManager_CreateAndWait_Timeout(t *testing.T) {
	mgr := NewManager(nil)

	start := time.Now()
	_, _, err := mgr.CreateAndWait(CreateRequest{
		AgentID: "agent-1",
		Type:    TypeDecision,
		Message: "Choose",
	}, 100*time.Millisecond)
	elapsed := time.Since(start)

	if err == nil {
		t.Error("CreateAndWait() should have timed out")
	}

	if elapsed < 90*time.Millisecond {
		t.Errorf("CreateAndWait() returned too quickly: %v", elapsed)
	}
}

func TestManager_Delete(t *testing.T) {
	mgr := NewManager(nil)

	sol, _ := mgr.Create(CreateRequest{
		AgentID: "agent-1",
		Type:    TypeInfo,
		Message: "Info",
	})

	err := mgr.Delete(sol.ID)
	if err != nil {
		t.Fatalf("Delete() error = %v", err)
	}

	_, err = mgr.Get(sol.ID)
	if err == nil {
		t.Error("Get() should have failed after delete")
	}
}

func TestManager_DismissAllForAgent(t *testing.T) {
	mgr := NewManager(nil)

	_, _ = mgr.Create(CreateRequest{
		AgentID: "agent-1",
		Type:    TypeBlocker,
		Message: "Blocker 1",
	})

	_, _ = mgr.Create(CreateRequest{
		AgentID: "agent-1",
		Type:    TypeBlocker,
		Message: "Blocker 2",
	})

	_, _ = mgr.Create(CreateRequest{
		AgentID: "agent-2",
		Type:    TypeBlocker,
		Message: "Other agent",
	})

	mgr.DismissAllForAgent("agent-1", "Agent stopped")

	pending := mgr.ListPending()
	if len(pending) != 1 {
		t.Errorf("ListPending() len = %d, want 1", len(pending))
	}

	if pending[0].AgentID != "agent-2" {
		t.Errorf("Remaining pending is for %s, want agent-2", pending[0].AgentID)
	}
}

func TestManager_Count(t *testing.T) {
	mgr := NewManager(nil)

	sol1, _ := mgr.Create(CreateRequest{
		AgentID: "agent-1",
		Type:    TypeBlocker,
		Message: "Blocker",
	})

	_, _ = mgr.Create(CreateRequest{
		AgentID: "agent-1",
		Type:    TypeInfo,
		Message: "Info",
	})

	_ = mgr.Respond(sol1.ID, RespondRequest{Response: "Fixed"})

	counts := mgr.Count()

	if counts[StatusPending] != 1 {
		t.Errorf("Count pending = %d, want 1", counts[StatusPending])
	}

	if counts[StatusResponded] != 1 {
		t.Errorf("Count responded = %d, want 1", counts[StatusResponded])
	}
}

func TestManager_GetByAgent(t *testing.T) {
	mgr := NewManager(nil)

	_, _ = mgr.Create(CreateRequest{
		AgentID: "agent-1",
		Type:    TypeBlocker,
		Message: "Blocker 1",
	})

	_, _ = mgr.Create(CreateRequest{
		AgentID: "agent-1",
		Type:    TypeInfo,
		Message: "Info 1",
	})

	_, _ = mgr.Create(CreateRequest{
		AgentID: "agent-2",
		Type:    TypeDecision,
		Message: "Decision",
	})

	agent1Sols := mgr.GetByAgent("agent-1")
	if len(agent1Sols) != 2 {
		t.Errorf("GetByAgent(agent-1) len = %d, want 2", len(agent1Sols))
	}

	agent2Sols := mgr.GetByAgent("agent-2")
	if len(agent2Sols) != 1 {
		t.Errorf("GetByAgent(agent-2) len = %d, want 1", len(agent2Sols))
	}
}
