package compose

import (
	"strings"
	"testing"
)

func TestGenerate(t *testing.T) {
	tests := []struct {
		name        string
		workers     int
		wantQueenSvc bool
		wantWorkers []string
		wantNoAgent string // Agent that should NOT be present
	}{
		{
			name:        "2 workers",
			workers:     2,
			wantQueenSvc: true,
			wantWorkers: []string{"drone-1:", "drone-2:"},
			wantNoAgent: "drone-3:",
		},
		{
			name:        "5 workers",
			workers:     5,
			wantQueenSvc: true,
			wantWorkers: []string{"drone-1:", "drone-2:", "drone-3:", "drone-4:", "drone-5:"},
			wantNoAgent: "drone-6:",
		},
		{
			name:        "0 workers",
			workers:     0,
			wantQueenSvc: true,
			wantWorkers: []string{},
			wantNoAgent: "drone-1:",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			output := Generate(tt.workers)

			// Check queen service is present
			if tt.wantQueenSvc && !strings.Contains(output, "queen:") {
				t.Error("Expected queen service to be present")
			}

			// Check redis service is present
			if !strings.Contains(output, "redis:") {
				t.Error("Expected redis service to be present")
			}

			// Check expected workers are present
			for _, worker := range tt.wantWorkers {
				if !strings.Contains(output, worker) {
					t.Errorf("Expected worker %q to be present", worker)
				}
			}

			// Check that unwanted agent is NOT present
			if tt.wantNoAgent != "" && strings.Contains(output, tt.wantNoAgent) {
				t.Errorf("Expected %q to NOT be present", tt.wantNoAgent)
			}

			// Check volumes are defined
			if !strings.Contains(output, "tools-cache:") {
				t.Error("Expected tools-cache volume to be defined")
			}
			if !strings.Contains(output, "pnpm-store:") {
				t.Error("Expected pnpm-store volume to be defined")
			}

			// Check network is defined
			if !strings.Contains(output, "hive-network:") {
				t.Error("Expected hive-network to be defined")
			}

			// Check build context is correct (relative to .hive/)
			if !strings.Contains(output, "context: .") {
				t.Error("Expected build context to be '.'")
			}
			if !strings.Contains(output, "dockerfile: docker/Dockerfile.node") {
				t.Error("Expected dockerfile path to be 'docker/Dockerfile.node'")
			}

			// Check volume paths are relative to .hive/
			if !strings.Contains(output, "./workspaces/queen:/workspace") {
				t.Error("Expected queen workspace path to be relative to .hive/")
			}
			if !strings.Contains(output, "../.git:/workspace-git") {
				t.Error("Expected .git path to be mounted at /workspace-git")
			}
		})
	}
}

func TestGenerateOutput(t *testing.T) {
	// Print sample output for visual verification
	output := Generate(2)
	t.Logf("Generated docker-compose.yml for 2 workers:\n%s", output)
}
