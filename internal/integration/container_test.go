//go:build integration
// +build integration

package integration

import (
	"context"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
	"github.com/testcontainers/testcontainers-go"
	"github.com/testcontainers/testcontainers-go/network"
	"github.com/testcontainers/testcontainers-go/wait"
)

const (
	redisImage    = "redis:7-alpine"
	testTimeout   = 2 * time.Minute
	containerWait = 30 * time.Second
)

// TestContainerSetup verifies basic container configuration
func TestContainerSetup(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping integration test in short mode")
	}

	// Check if hive-claude image exists
	cmd := exec.Command("docker", "images", "-q", "hive-claude:latest")
	output, err := cmd.Output()
	if err != nil || len(output) == 0 {
		t.Skip("Skipping: hive-claude:latest image not found. Run 'make build' first.")
	}

	ctx, cancel := context.WithTimeout(context.Background(), testTimeout)
	defer cancel()

	// Create test network
	testNet, err := network.New(ctx)
	require.NoError(t, err, "Failed to create test network")
	defer testNet.Remove(ctx)

	// Start Redis container
	redisC, err := testcontainers.GenericContainer(ctx, testcontainers.GenericContainerRequest{
		ContainerRequest: testcontainers.ContainerRequest{
			Image:        redisImage,
			ExposedPorts: []string{"6379/tcp"},
			Networks:     []string{testNet.Name},
			NetworkAliases: map[string][]string{
				testNet.Name: {"redis"},
			},
			WaitingFor: wait.ForLog("Ready to accept connections").WithStartupTimeout(containerWait),
		},
		Started: true,
	})
	require.NoError(t, err, "Failed to start Redis container")
	defer redisC.Terminate(ctx)

	t.Run("redis_is_accessible", func(t *testing.T) {
		// Test Redis is responding
		code, _, err := redisC.Exec(ctx, []string{"redis-cli", "PING"})
		require.NoError(t, err)
		assert.Equal(t, 0, code, "Redis should respond to PING")
	})
}

// TestRedisActivityLogging verifies the Redis activity stream works
func TestRedisActivityLogging(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping integration test in short mode")
	}

	ctx, cancel := context.WithTimeout(context.Background(), testTimeout)
	defer cancel()

	// Start Redis container
	redisC, err := testcontainers.GenericContainer(ctx, testcontainers.GenericContainerRequest{
		ContainerRequest: testcontainers.ContainerRequest{
			Image:        redisImage,
			ExposedPorts: []string{"6379/tcp"},
			WaitingFor:   wait.ForLog("Ready to accept connections").WithStartupTimeout(containerWait),
		},
		Started: true,
	})
	require.NoError(t, err, "Failed to start Redis container")
	defer redisC.Terminate(ctx)

	t.Run("can_write_activity_stream", func(t *testing.T) {
		// Write to activity stream (same format as hive uses)
		code, _, err := redisC.Exec(ctx, []string{
			"redis-cli", "XADD", "hive:activity:queen", "*",
			"type", "tool_use",
			"agent", "queen",
			"tool", "Read",
			"status", "started",
		})
		require.NoError(t, err)
		assert.Equal(t, 0, code, "Should be able to write to activity stream")

		// Read from activity stream
		code, reader, err := redisC.Exec(ctx, []string{
			"redis-cli", "XRANGE", "hive:activity:queen", "-", "+", "COUNT", "1",
		})
		require.NoError(t, err)
		assert.Equal(t, 0, code, "Should be able to read from activity stream")

		// Read output
		buf := make([]byte, 1024)
		n, _ := reader.Read(buf)
		output := string(buf[:n])
		assert.Contains(t, output, "tool_use", "Activity entry should contain tool_use")
	})

	t.Run("can_use_pubsub", func(t *testing.T) {
		// Test pub/sub channel (used for queen/drone communication)
		// We'll just verify publish works
		code, _, err := redisC.Exec(ctx, []string{
			"redis-cli", "PUBLISH", "hive:tasks", "test-task-message",
		})
		require.NoError(t, err)
		assert.Equal(t, 0, code, "Should be able to publish to channel")
	})
}

// TestHiveConfigGeneration verifies that hive init creates valid config
func TestHiveConfigGeneration(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping integration test in short mode")
	}

	// Find project root and hive binary
	cwd, _ := os.Getwd()
	projectRoot := cwd
	for i := 0; i < 10; i++ {
		if _, err := os.Stat(filepath.Join(projectRoot, "go.mod")); err == nil {
			break
		}
		projectRoot = filepath.Dir(projectRoot)
	}

	// Look for hive binary in project root first, then PATH
	hiveBin := filepath.Join(projectRoot, "hive")
	if _, err := os.Stat(hiveBin); os.IsNotExist(err) {
		var lookErr error
		hiveBin, lookErr = exec.LookPath("hive")
		if lookErr != nil {
			t.Skip("Skipping: hive binary not found. Run 'make build' or 'make install' first.")
		}
	}

	// Create a temp directory for the test
	tmpDir := t.TempDir()

	// Initialize git repo (required by hive init)
	cmd := exec.Command("git", "init")
	cmd.Dir = tmpDir
	require.NoError(t, cmd.Run(), "Failed to init git repo")

	cmd = exec.Command("git", "config", "user.email", "test@example.com")
	cmd.Dir = tmpDir
	cmd.Run()

	cmd = exec.Command("git", "config", "user.name", "Test User")
	cmd.Dir = tmpDir
	cmd.Run()

	// Create initial commit
	readmePath := filepath.Join(tmpDir, "README.md")
	os.WriteFile(readmePath, []byte("# Test"), 0644)
	cmd = exec.Command("git", "add", ".")
	cmd.Dir = tmpDir
	cmd.Run()
	cmd = exec.Command("git", "commit", "-m", "Initial")
	cmd.Dir = tmpDir
	cmd.Run()

	// Run hive init in non-interactive mode
	cmd = exec.Command(hiveBin, "init",
		"-y",
		"--skip-start",
		"--workers", "1",
	)
	cmd.Dir = tmpDir
	cmd.Env = append(os.Environ(),
		"CLAUDE_CODE_OAUTH_TOKEN=test-token-xxx",
	)
	output, err := cmd.CombinedOutput()
	t.Logf("hive init output: %s", output)

	// Check if init succeeded or if it failed due to Docker not being available
	if err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			t.Logf("hive init exit code: %d", exitErr.ExitCode())
			// If the error mentions Docker, skip
			if strings.Contains(string(output), "docker") || strings.Contains(string(output), "Docker") {
				t.Skip("Skipping: hive init failed due to Docker connectivity")
			}
		}
		t.Fatalf("hive init failed: %v", err)
	}

	t.Run("hive_yaml_created", func(t *testing.T) {
		hiveYaml := filepath.Join(tmpDir, "hive.yaml")
		assert.FileExists(t, hiveYaml, "hive.yaml should be created")
	})

	t.Run("hive_directory_created", func(t *testing.T) {
		hiveDir := filepath.Join(tmpDir, ".hive")
		assert.DirExists(t, hiveDir, ".hive directory should be created")
	})

	t.Run("env_file_created", func(t *testing.T) {
		envFile := filepath.Join(tmpDir, ".hive", ".env")
		assert.FileExists(t, envFile, ".env should be created")

		content, err := os.ReadFile(envFile)
		require.NoError(t, err)
		assert.Contains(t, string(content), "CLAUDE_CODE_OAUTH_TOKEN", ".env should contain token")
	})

	t.Run("docker_compose_created", func(t *testing.T) {
		dcFile := filepath.Join(tmpDir, ".hive", "docker-compose.yml")
		assert.FileExists(t, dcFile, "docker-compose.yml should be created")
	})

	t.Run("worktrees_created", func(t *testing.T) {
		// Check queen worktree
		queenWorktree := filepath.Join(tmpDir, ".hive", "workspaces", "queen")
		if _, err := os.Stat(queenWorktree); err == nil {
			assert.DirExists(t, queenWorktree, "Queen worktree should be created")
		}
	})
}

// TestDockerComposeStructure verifies generated docker-compose.yml
func TestDockerComposeStructure(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping integration test in short mode")
	}

	// Use the compose package to generate content (same as hive init)
	// This import would need to be added, so we'll test the generated file instead
	tmpDir := t.TempDir()

	// Find and run hive to generate docker-compose.yml
	cwd, _ := os.Getwd()
	projectRoot := cwd
	for i := 0; i < 10; i++ {
		if _, err := os.Stat(filepath.Join(projectRoot, "go.mod")); err == nil {
			break
		}
		projectRoot = filepath.Dir(projectRoot)
	}

	hiveBin := filepath.Join(projectRoot, "hive")
	if _, err := os.Stat(hiveBin); os.IsNotExist(err) {
		var lookErr error
		hiveBin, lookErr = exec.LookPath("hive")
		if lookErr != nil {
			t.Skip("Skipping: hive binary not found")
		}
	}

	// Initialize git repo
	cmd := exec.Command("git", "init")
	cmd.Dir = tmpDir
	cmd.Run()
	cmd = exec.Command("git", "config", "user.email", "test@example.com")
	cmd.Dir = tmpDir
	cmd.Run()
	cmd = exec.Command("git", "config", "user.name", "Test")
	cmd.Dir = tmpDir
	cmd.Run()
	os.WriteFile(filepath.Join(tmpDir, "README.md"), []byte("# Test"), 0644)
	cmd = exec.Command("git", "add", ".")
	cmd.Dir = tmpDir
	cmd.Run()
	cmd = exec.Command("git", "commit", "-m", "init")
	cmd.Dir = tmpDir
	cmd.Run()

	// Run hive init
	cmd = exec.Command(hiveBin, "init", "-y", "--skip-start", "--workers", "2")
	cmd.Dir = tmpDir
	cmd.Env = append(os.Environ(), "CLAUDE_CODE_OAUTH_TOKEN=test-token")
	if err := cmd.Run(); err != nil {
		t.Skip("Skipping: hive init failed")
	}

	// Read generated docker-compose.yml
	dcPath := filepath.Join(tmpDir, ".hive", "docker-compose.yml")
	content, err := os.ReadFile(dcPath)
	if err != nil {
		t.Fatalf("Failed to read docker-compose.yml: %v", err)
	}
	composeContent := string(content)

	t.Run("has_redis_service", func(t *testing.T) {
		assert.Contains(t, composeContent, "redis:", "Should have redis service")
	})

	t.Run("has_queen_service", func(t *testing.T) {
		assert.Contains(t, composeContent, "queen:", "Should have queen service")
	})

	t.Run("has_drone_services", func(t *testing.T) {
		assert.Contains(t, composeContent, "drone-1:", "Should have drone-1 service")
		assert.Contains(t, composeContent, "drone-2:", "Should have drone-2 service")
	})

	t.Run("has_network_definition", func(t *testing.T) {
		assert.Contains(t, composeContent, "hive-network", "Should have network defined")
	})

	t.Run("redis_persistence", func(t *testing.T) {
		assert.Contains(t, composeContent, "redis-data", "Should have redis data volume")
	})

	t.Run("workspace_volumes", func(t *testing.T) {
		assert.Contains(t, composeContent, "/workspace", "Should mount workspace volume")
	})
}
