package task

import (
	"sync"
	"testing"
	"time"
)

func TestManager_Create(t *testing.T) {
	events := make([]TaskEvent, 0)
	var mu sync.Mutex

	mgr := NewManager(func(e TaskEvent) {
		mu.Lock()
		events = append(events, e)
		mu.Unlock()
	})

	task, err := mgr.Create(CreateTaskRequest{
		AgentID:   "agent-1",
		AgentName: "Agent One",
		Title:     "Test Task",
		Context:   "Testing context",
		Steps: []CreateStepRequest{
			{
				Action:   "Do something",
				DoD:      []string{"Something is done"},
				Autonomy: AutonomyFull,
			},
			{
				Action:   "Do another thing",
				DoD:      []string{"Another thing is done", "And validated"},
				Autonomy: AutonomyValidateBeforeNext,
			},
		},
	})

	if err != nil {
		t.Fatalf("Create() error = %v", err)
	}

	if task.ID == "" {
		t.Error("Create() task.ID is empty")
	}

	if task.Status != TaskStatusAssigned {
		t.Errorf("Create() task.Status = %s, want assigned", task.Status)
	}

	if len(task.Plan.Steps) != 2 {
		t.Errorf("Create() len(steps) = %d, want 2", len(task.Plan.Steps))
	}

	// Check event was emitted
	time.Sleep(10 * time.Millisecond)
	mu.Lock()
	defer mu.Unlock()
	if len(events) != 1 {
		t.Errorf("Expected 1 event, got %d", len(events))
	}
	if events[0].Type != "created" {
		t.Errorf("Event type = %s, want created", events[0].Type)
	}
}

func TestManager_Create_Validation(t *testing.T) {
	mgr := NewManager(nil)

	tests := []struct {
		name    string
		req     CreateTaskRequest
		wantErr bool
	}{
		{
			name:    "missing agent_id",
			req:     CreateTaskRequest{Title: "Test", Steps: []CreateStepRequest{{Action: "Do", DoD: []string{"Done"}}}},
			wantErr: true,
		},
		{
			name:    "missing title",
			req:     CreateTaskRequest{AgentID: "a1", Steps: []CreateStepRequest{{Action: "Do", DoD: []string{"Done"}}}},
			wantErr: true,
		},
		{
			name:    "missing steps",
			req:     CreateTaskRequest{AgentID: "a1", Title: "Test"},
			wantErr: true,
		},
		{
			name:    "step missing action",
			req:     CreateTaskRequest{AgentID: "a1", Title: "Test", Steps: []CreateStepRequest{{DoD: []string{"Done"}}}},
			wantErr: true,
		},
		{
			name:    "step missing dod",
			req:     CreateTaskRequest{AgentID: "a1", Title: "Test", Steps: []CreateStepRequest{{Action: "Do"}}},
			wantErr: true,
		},
		{
			name:    "valid request",
			req:     CreateTaskRequest{AgentID: "a1", Title: "Test", Steps: []CreateStepRequest{{Action: "Do", DoD: []string{"Done"}}}},
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

func TestManager_Start(t *testing.T) {
	mgr := NewManager(nil)

	task, _ := mgr.Create(CreateTaskRequest{
		AgentID: "agent-1",
		Title:   "Test Task",
		Steps: []CreateStepRequest{
			{Action: "Step 1", DoD: []string{"Done"}},
			{Action: "Step 2", DoD: []string{"Done"}},
		},
	})

	err := mgr.Start(task.ID)
	if err != nil {
		t.Fatalf("Start() error = %v", err)
	}

	updated, _ := mgr.Get(task.ID)
	if updated.Status != TaskStatusInProgress {
		t.Errorf("Start() status = %s, want in_progress", updated.Status)
	}

	if updated.CurrentStep != 1 {
		t.Errorf("Start() current_step = %d, want 1", updated.CurrentStep)
	}

	if updated.Plan.Steps[0].Status != StepStatusInProgress {
		t.Errorf("Start() step[0].status = %s, want in_progress", updated.Plan.Steps[0].Status)
	}
}

func TestManager_UpdateStep(t *testing.T) {
	mgr := NewManager(nil)

	task, _ := mgr.Create(CreateTaskRequest{
		AgentID: "agent-1",
		Title:   "Test Task",
		Steps: []CreateStepRequest{
			{Action: "Step 1", DoD: []string{"Done"}},
			{Action: "Step 2", DoD: []string{"Done"}},
		},
	})

	_ = mgr.Start(task.ID)

	// Complete first step
	err := mgr.UpdateStep(task.ID, 1, UpdateStepRequest{
		Status: StepStatusCompleted,
		Result: "Completed successfully",
	})
	if err != nil {
		t.Fatalf("UpdateStep() error = %v", err)
	}

	updated, _ := mgr.Get(task.ID)
	if updated.CurrentStep != 2 {
		t.Errorf("UpdateStep() current_step = %d, want 2", updated.CurrentStep)
	}

	if updated.Plan.Steps[0].Status != StepStatusCompleted {
		t.Errorf("UpdateStep() step[0].status = %s, want completed", updated.Plan.Steps[0].Status)
	}

	if updated.Plan.Steps[1].Status != StepStatusInProgress {
		t.Errorf("UpdateStep() step[1].status = %s, want in_progress", updated.Plan.Steps[1].Status)
	}
}

func TestManager_Complete(t *testing.T) {
	mgr := NewManager(nil)

	task, _ := mgr.Create(CreateTaskRequest{
		AgentID: "agent-1",
		Title:   "Test Task",
		Steps:   []CreateStepRequest{{Action: "Step 1", DoD: []string{"Done"}}},
	})

	_ = mgr.Start(task.ID)

	err := mgr.Complete(task.ID, CompleteTaskRequest{
		Result: "All done!",
		Artifacts: []Artifact{
			{Type: "mr", Name: "MR-123", URL: "https://gitlab.com/mr/123"},
		},
	})
	if err != nil {
		t.Fatalf("Complete() error = %v", err)
	}

	updated, _ := mgr.Get(task.ID)
	if updated.Status != TaskStatusCompleted {
		t.Errorf("Complete() status = %s, want completed", updated.Status)
	}

	if len(updated.Artifacts) != 1 {
		t.Errorf("Complete() len(artifacts) = %d, want 1", len(updated.Artifacts))
	}
}

func TestManager_Fail(t *testing.T) {
	mgr := NewManager(nil)

	task, _ := mgr.Create(CreateTaskRequest{
		AgentID: "agent-1",
		Title:   "Test Task",
		Steps:   []CreateStepRequest{{Action: "Step 1", DoD: []string{"Done"}}},
	})

	_ = mgr.Start(task.ID)

	err := mgr.Fail(task.ID, FailTaskRequest{
		Error: "Something went wrong",
	})
	if err != nil {
		t.Fatalf("Fail() error = %v", err)
	}

	updated, _ := mgr.Get(task.ID)
	if updated.Status != TaskStatusFailed {
		t.Errorf("Fail() status = %s, want failed", updated.Status)
	}

	if updated.Error != "Something went wrong" {
		t.Errorf("Fail() error = %s, want 'Something went wrong'", updated.Error)
	}
}

func TestManager_Cancel(t *testing.T) {
	mgr := NewManager(nil)

	task, _ := mgr.Create(CreateTaskRequest{
		AgentID: "agent-1",
		Title:   "Test Task",
		Steps:   []CreateStepRequest{{Action: "Step 1", DoD: []string{"Done"}}},
	})

	err := mgr.Cancel(task.ID, "User cancelled")
	if err != nil {
		t.Fatalf("Cancel() error = %v", err)
	}

	updated, _ := mgr.Get(task.ID)
	if updated.Status != TaskStatusCancelled {
		t.Errorf("Cancel() status = %s, want cancelled", updated.Status)
	}
}

func TestManager_GetByAgent(t *testing.T) {
	mgr := NewManager(nil)

	task1, _ := mgr.Create(CreateTaskRequest{
		AgentID: "agent-1",
		Title:   "Task 1",
		Steps:   []CreateStepRequest{{Action: "Step", DoD: []string{"Done"}}},
	})

	_, _ = mgr.Create(CreateTaskRequest{
		AgentID: "agent-2",
		Title:   "Task 2",
		Steps:   []CreateStepRequest{{Action: "Step", DoD: []string{"Done"}}},
	})

	task := mgr.GetByAgent("agent-1")
	if task == nil {
		t.Fatal("GetByAgent() returned nil")
	}

	if task.ID != task1.ID {
		t.Errorf("GetByAgent() id = %s, want %s", task.ID, task1.ID)
	}
}

func TestManager_List(t *testing.T) {
	mgr := NewManager(nil)

	_, _ = mgr.Create(CreateTaskRequest{
		AgentID: "agent-1",
		Title:   "Task 1",
		Steps:   []CreateStepRequest{{Action: "Step", DoD: []string{"Done"}}},
	})

	task2, _ := mgr.Create(CreateTaskRequest{
		AgentID: "agent-2",
		Title:   "Task 2",
		Steps:   []CreateStepRequest{{Action: "Step", DoD: []string{"Done"}}},
	})
	_ = mgr.Start(task2.ID)

	// All tasks
	all := mgr.List("", "")
	if len(all) != 2 {
		t.Errorf("List() len = %d, want 2", len(all))
	}

	// Filter by agent
	agent1Tasks := mgr.List("agent-1", "")
	if len(agent1Tasks) != 1 {
		t.Errorf("List(agent-1) len = %d, want 1", len(agent1Tasks))
	}

	// Filter by status
	inProgress := mgr.List("", TaskStatusInProgress)
	if len(inProgress) != 1 {
		t.Errorf("List(in_progress) len = %d, want 1", len(inProgress))
	}
}

func TestManager_Delete(t *testing.T) {
	mgr := NewManager(nil)

	task, _ := mgr.Create(CreateTaskRequest{
		AgentID: "agent-1",
		Title:   "Test Task",
		Steps:   []CreateStepRequest{{Action: "Step", DoD: []string{"Done"}}},
	})

	err := mgr.Delete(task.ID)
	if err != nil {
		t.Fatalf("Delete() error = %v", err)
	}

	_, err = mgr.Get(task.ID)
	if err == nil {
		t.Error("Get() should have failed after delete")
	}
}

func TestManager_GetCurrentStep(t *testing.T) {
	mgr := NewManager(nil)

	task, _ := mgr.Create(CreateTaskRequest{
		AgentID: "agent-1",
		Title:   "Test Task",
		Steps: []CreateStepRequest{
			{Action: "Step 1", DoD: []string{"Done"}},
			{Action: "Step 2", DoD: []string{"Done"}},
		},
	})

	_ = mgr.Start(task.ID)

	step, err := mgr.GetCurrentStep(task.ID)
	if err != nil {
		t.Fatalf("GetCurrentStep() error = %v", err)
	}

	if step.ID != 1 {
		t.Errorf("GetCurrentStep() id = %d, want 1", step.ID)
	}

	if step.Action != "Step 1" {
		t.Errorf("GetCurrentStep() action = %s, want 'Step 1'", step.Action)
	}
}

func TestManager_CancelAllForAgent(t *testing.T) {
	mgr := NewManager(nil)

	_, _ = mgr.Create(CreateTaskRequest{
		AgentID: "agent-1",
		Title:   "Task 1",
		Steps:   []CreateStepRequest{{Action: "Step", DoD: []string{"Done"}}},
	})

	_, _ = mgr.Create(CreateTaskRequest{
		AgentID: "agent-1",
		Title:   "Task 2",
		Steps:   []CreateStepRequest{{Action: "Step", DoD: []string{"Done"}}},
	})

	mgr.CancelAllForAgent("agent-1", "Agent stopped")

	tasks := mgr.GetTasksForAgent("agent-1")
	for _, task := range tasks {
		if task.Status != TaskStatusCancelled {
			t.Errorf("Task %s status = %s, want cancelled", task.ID, task.Status)
		}
	}
}
