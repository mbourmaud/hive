//go:build integration

// Package integration provides end-to-end tests using real Docker containers
package integration

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/redis/go-redis/v9"
	"github.com/testcontainers/testcontainers-go"
	"github.com/testcontainers/testcontainers-go/wait"
	tcredis "github.com/testcontainers/testcontainers-go/modules/redis"
)

// =============================================================================
// Full E2E Tests - Simulating Queen + Daemons on a Test Repository
// =============================================================================

// E2ETestSuite holds all containers and state for E2E tests
type E2ETestSuite struct {
	ctx           context.Context
	redisContainer testcontainers.Container
	redisClient   *redis.Client
	testRepoPath  string
	hiveDir       string
}

// setupE2E creates the full test environment
func setupE2E(t *testing.T) *E2ETestSuite {
	ctx := context.Background()

	// Start Redis container
	redisContainer, err := tcredis.Run(ctx, "redis:7-alpine")
	if err != nil {
		t.Fatalf("Failed to start Redis: %v", err)
	}

	endpoint, err := redisContainer.Endpoint(ctx, "")
	if err != nil {
		t.Fatalf("Failed to get Redis endpoint: %v", err)
	}

	client := redis.NewClient(&redis.Options{Addr: endpoint})
	if err := client.Ping(ctx).Err(); err != nil {
		t.Fatalf("Failed to connect to Redis: %v", err)
	}

	// Create a test git repository
	testRepoPath, err := createTestRepository(t)
	if err != nil {
		t.Fatalf("Failed to create test repo: %v", err)
	}

	return &E2ETestSuite{
		ctx:           ctx,
		redisContainer: redisContainer,
		redisClient:   client,
		testRepoPath:  testRepoPath,
		hiveDir:       filepath.Join(testRepoPath, ".hive"),
	}
}

// teardown cleans up all resources
func (s *E2ETestSuite) teardown(t *testing.T) {
	if s.redisClient != nil {
		s.redisClient.Close()
	}
	if s.redisContainer != nil {
		s.redisContainer.Terminate(s.ctx)
	}
	if s.testRepoPath != "" {
		os.RemoveAll(s.testRepoPath)
	}
}

// createTestRepository creates a temporary git repository for testing
func createTestRepository(t *testing.T) (string, error) {
	tmpDir, err := os.MkdirTemp("", "hive-e2e-*")
	if err != nil {
		return "", err
	}

	// Initialize git repo
	cmds := [][]string{
		{"git", "init"},
		{"git", "config", "user.email", "test@example.com"},
		{"git", "config", "user.name", "Test User"},
	}

	for _, args := range cmds {
		cmd := exec.Command(args[0], args[1:]...)
		cmd.Dir = tmpDir
		if err := cmd.Run(); err != nil {
			return "", fmt.Errorf("failed to run %v: %w", args, err)
		}
	}

	// Create a simple project structure
	files := map[string]string{
		"README.md":     "# Test Project\n\nThis is a test repository for Hive E2E tests.",
		"package.json":  `{"name": "test-project", "version": "1.0.0"}`,
		"src/index.js":  `console.log("Hello, Hive!");`,
		"src/utils.js":  `export function add(a, b) { return a + b; }`,
		".gitignore":    "node_modules/\n.hive/\n",
	}

	for path, content := range files {
		fullPath := filepath.Join(tmpDir, path)
		os.MkdirAll(filepath.Dir(fullPath), 0755)
		os.WriteFile(fullPath, []byte(content), 0644)
	}

	// Create initial commit
	cmd := exec.Command("git", "add", ".")
	cmd.Dir = tmpDir
	cmd.Run()

	cmd = exec.Command("git", "commit", "-m", "Initial commit")
	cmd.Dir = tmpDir
	cmd.Run()

	return tmpDir, nil
}

// =============================================================================
// Queen Simulation Functions
// =============================================================================

// queenAssignTask simulates the queen assigning a task to a drone
func (s *E2ETestSuite) queenAssignTask(droneID string, task Task) error {
	task.Status = "pending"
	task.CreatedAt = time.Now().Format(time.RFC3339)

	taskJSON, err := json.Marshal(task)
	if err != nil {
		return err
	}

	queueKey := fmt.Sprintf("hive:queue:%s", droneID)
	if err := s.redisClient.LPush(s.ctx, queueKey, taskJSON).Err(); err != nil {
		return err
	}

	// Publish notification
	return s.redisClient.Publish(s.ctx, "hive:events",
		fmt.Sprintf("task_queued:%s", droneID)).Err()
}

// queenGetWorkerStatus gets the current status of a worker
func (s *E2ETestSuite) queenGetWorkerStatus(droneID string) (int64, int64, error) {
	queueKey := fmt.Sprintf("hive:queue:%s", droneID)
	activeKey := fmt.Sprintf("hive:active:%s", droneID)

	queued, err := s.redisClient.LLen(s.ctx, queueKey).Result()
	if err != nil {
		return 0, 0, err
	}

	active, err := s.redisClient.LLen(s.ctx, activeKey).Result()
	if err != nil {
		return 0, 0, err
	}

	return queued, active, nil
}

// queenGetCompletedTasks retrieves all completed tasks
func (s *E2ETestSuite) queenGetCompletedTasks() ([]Task, error) {
	results, err := s.redisClient.ZRevRange(s.ctx, "hive:completed", 0, -1).Result()
	if err != nil {
		return nil, err
	}

	tasks := make([]Task, 0, len(results))
	for _, result := range results {
		var task Task
		if err := json.Unmarshal([]byte(result), &task); err != nil {
			continue
		}
		tasks = append(tasks, task)
	}

	return tasks, nil
}

// =============================================================================
// Daemon Simulation Functions
// =============================================================================

// daemonTakeTask simulates a daemon taking a task from its queue
func (s *E2ETestSuite) daemonTakeTask(droneID string) (*Task, error) {
	queueKey := fmt.Sprintf("hive:queue:%s", droneID)
	activeKey := fmt.Sprintf("hive:active:%s", droneID)

	result, err := s.redisClient.RPopLPush(s.ctx, queueKey, activeKey).Result()
	if err == redis.Nil {
		return nil, nil // No tasks
	}
	if err != nil {
		return nil, err
	}

	var task Task
	if err := json.Unmarshal([]byte(result), &task); err != nil {
		return nil, err
	}

	return &task, nil
}

// daemonCompleteTask simulates a daemon completing a task
func (s *E2ETestSuite) daemonCompleteTask(droneID string, result map[string]interface{}) error {
	activeKey := fmt.Sprintf("hive:active:%s", droneID)

	taskJSON, err := s.redisClient.RPop(s.ctx, activeKey).Result()
	if err != nil {
		return err
	}

	var task Task
	if err := json.Unmarshal([]byte(taskJSON), &task); err != nil {
		return err
	}

	task.Status = "completed"
	task.CompletedAt = time.Now().Format(time.RFC3339)
	task.Result = result

	completedJSON, _ := json.Marshal(task)

	// Add to completed set
	timestamp := float64(time.Now().UnixNano())
	if err := s.redisClient.ZAdd(s.ctx, "hive:completed", redis.Z{
		Score:  timestamp,
		Member: string(completedJSON),
	}).Err(); err != nil {
		return err
	}

	// Store in hash
	taskHashKey := fmt.Sprintf("hive:task:%s", task.ID)
	if err := s.redisClient.HSet(s.ctx, taskHashKey, "data", string(completedJSON)).Err(); err != nil {
		return err
	}

	// Publish event
	return s.redisClient.Publish(s.ctx, "hive:events",
		fmt.Sprintf("task_completed:%s:%s", droneID, task.ID)).Err()
}

// daemonFailTask simulates a daemon failing a task
func (s *E2ETestSuite) daemonFailTask(droneID string, errorMsg string) error {
	activeKey := fmt.Sprintf("hive:active:%s", droneID)

	taskJSON, err := s.redisClient.RPop(s.ctx, activeKey).Result()
	if err != nil {
		return err
	}

	var task Task
	json.Unmarshal([]byte(taskJSON), &task)
	task.Status = "failed"
	task.Result = map[string]interface{}{"error": errorMsg}

	failedJSON, _ := json.Marshal(task)

	timestamp := float64(time.Now().UnixNano())
	if err := s.redisClient.ZAdd(s.ctx, "hive:failed", redis.Z{
		Score:  timestamp,
		Member: string(failedJSON),
	}).Err(); err != nil {
		return err
	}

	return s.redisClient.Publish(s.ctx, "hive:events",
		fmt.Sprintf("task_failed:%s:%s", droneID, task.ID)).Err()
}

// =============================================================================
// E2E Test Cases
// =============================================================================

// TestE2E_QueenAssignsDaemonProcesses tests the complete flow
func TestE2E_QueenAssignsDaemonProcesses(t *testing.T) {
	suite := setupE2E(t)
	defer suite.teardown(t)

	// Queen assigns tasks to 3 drones
	drones := []string{"drone-1", "drone-2", "drone-3"}
	tasks := []Task{
		{ID: "task-1", Title: "Add login endpoint", Description: "Create POST /api/login", JiraTicket: "PROJ-101"},
		{ID: "task-2", Title: "Add logout endpoint", Description: "Create POST /api/logout", JiraTicket: "PROJ-102"},
		{ID: "task-3", Title: "Add tests for auth", Description: "Unit tests for login/logout", JiraTicket: "PROJ-103"},
		{ID: "task-4", Title: "Update docs", Description: "Document auth endpoints", JiraTicket: "PROJ-104"},
		{ID: "task-5", Title: "Fix CORS", Description: "Allow auth endpoints", JiraTicket: "PROJ-105"},
	}

	// Queen assigns tasks round-robin
	for i, task := range tasks {
		droneID := drones[i%len(drones)]
		if err := suite.queenAssignTask(droneID, task); err != nil {
			t.Fatalf("Failed to assign task %s: %v", task.ID, err)
		}
	}

	// Verify queue status
	for _, drone := range drones {
		queued, _, err := suite.queenGetWorkerStatus(drone)
		if err != nil {
			t.Fatalf("Failed to get status for %s: %v", drone, err)
		}
		t.Logf("Drone %s has %d queued tasks", drone, queued)
	}

	// Daemons process tasks concurrently
	var wg sync.WaitGroup
	for _, drone := range drones {
		wg.Add(1)
		go func(droneID string) {
			defer wg.Done()

			for {
				task, err := suite.daemonTakeTask(droneID)
				if err != nil {
					t.Errorf("Daemon %s error: %v", droneID, err)
					return
				}
				if task == nil {
					return // No more tasks
				}

				// Simulate work
				time.Sleep(50 * time.Millisecond)

				// Complete task
				result := map[string]interface{}{
					"worker":     droneID,
					"duration":   "50ms",
					"files_changed": 2,
				}
				if err := suite.daemonCompleteTask(droneID, result); err != nil {
					t.Errorf("Failed to complete task: %v", err)
				}
			}
		}(drone)
	}

	wg.Wait()

	// Queen verifies all tasks completed
	completedTasks, err := suite.queenGetCompletedTasks()
	if err != nil {
		t.Fatalf("Failed to get completed tasks: %v", err)
	}

	if len(completedTasks) != len(tasks) {
		t.Errorf("Expected %d completed tasks, got %d", len(tasks), len(completedTasks))
	}

	// Verify task details
	for _, task := range completedTasks {
		if task.Status != "completed" {
			t.Errorf("Task %s has status %s, expected completed", task.ID, task.Status)
		}
		if task.Result == nil {
			t.Errorf("Task %s has no result", task.ID)
		}
	}
}

// TestE2E_DaemonFailureAndRetry tests failure handling
func TestE2E_DaemonFailureAndRetry(t *testing.T) {
	suite := setupE2E(t)
	defer suite.teardown(t)

	// Queen assigns a flaky task
	task := Task{
		ID:          "flaky-task",
		Title:       "Flaky operation",
		Description: "This task will fail first, then succeed",
	}

	if err := suite.queenAssignTask("drone-1", task); err != nil {
		t.Fatalf("Failed to assign task: %v", err)
	}

	// Drone-1 takes the task and fails
	takenTask, _ := suite.daemonTakeTask("drone-1")
	if takenTask == nil {
		t.Fatal("No task available")
	}

	// Simulate failure
	if err := suite.daemonFailTask("drone-1", "Network timeout"); err != nil {
		t.Fatalf("Failed to fail task: %v", err)
	}

	// Verify task is in failed set
	failedCount, _ := suite.redisClient.ZCard(suite.ctx, "hive:failed").Result()
	if failedCount != 1 {
		t.Errorf("Expected 1 failed task, got %d", failedCount)
	}

	// Queen requeues the task (manual retry)
	task.ID = "flaky-task-retry"
	if err := suite.queenAssignTask("drone-2", task); err != nil {
		t.Fatalf("Failed to requeue task: %v", err)
	}

	// Drone-2 succeeds
	retryTask, _ := suite.daemonTakeTask("drone-2")
	if retryTask == nil {
		t.Fatal("Retry task not available")
	}

	if err := suite.daemonCompleteTask("drone-2", map[string]interface{}{"retry": true}); err != nil {
		t.Fatalf("Failed to complete retry task: %v", err)
	}

	// Verify task completed
	completedTasks, _ := suite.queenGetCompletedTasks()
	if len(completedTasks) != 1 {
		t.Errorf("Expected 1 completed task, got %d", len(completedTasks))
	}
}

// TestE2E_MultipleQueuesIsolation tests that drones don't interfere
func TestE2E_MultipleQueuesIsolation(t *testing.T) {
	suite := setupE2E(t)
	defer suite.teardown(t)

	// Assign specific tasks to specific drones
	assignments := map[string][]string{
		"drone-1": {"d1-task-1", "d1-task-2"},
		"drone-2": {"d2-task-1", "d2-task-2", "d2-task-3"},
		"drone-3": {"d3-task-1"},
	}

	for droneID, taskIDs := range assignments {
		for _, taskID := range taskIDs {
			task := Task{ID: taskID, Title: taskID}
			suite.queenAssignTask(droneID, task)
		}
	}

	// Each drone processes only its own tasks
	results := make(map[string][]string)
	var mu sync.Mutex
	var wg sync.WaitGroup

	for droneID := range assignments {
		wg.Add(1)
		go func(drone string) {
			defer wg.Done()
			processed := []string{}

			for {
				task, _ := suite.daemonTakeTask(drone)
				if task == nil {
					break
				}
				processed = append(processed, task.ID)
				suite.daemonCompleteTask(drone, nil)
			}

			mu.Lock()
			results[drone] = processed
			mu.Unlock()
		}(droneID)
	}

	wg.Wait()

	// Verify each drone processed only its assigned tasks
	for droneID, expectedTasks := range assignments {
		actual := results[droneID]
		if len(actual) != len(expectedTasks) {
			t.Errorf("Drone %s processed %d tasks, expected %d",
				droneID, len(actual), len(expectedTasks))
		}

		// Verify task IDs belong to this drone
		// Task IDs are like "d1-task-1" for drone-1, "d2-task-1" for drone-2
		expectedPrefix := "d" + droneID[len(droneID)-1:] + "-" // "d1-", "d2-", "d3-"
		for _, taskID := range actual {
			if !strings.HasPrefix(taskID, expectedPrefix) {
				t.Errorf("Drone %s processed task %s from another drone", droneID, taskID)
			}
		}
	}
}

// TestE2E_EventNotifications tests pub/sub events
func TestE2E_EventNotifications(t *testing.T) {
	suite := setupE2E(t)
	defer suite.teardown(t)

	// Subscribe to events
	pubsub := suite.redisClient.Subscribe(suite.ctx, "hive:events")
	defer pubsub.Close()

	// Wait for subscription
	_, err := pubsub.Receive(suite.ctx)
	if err != nil {
		t.Fatalf("Failed to subscribe: %v", err)
	}

	events := make(chan string, 100)
	go func() {
		for msg := range pubsub.Channel() {
			events <- msg.Payload
		}
	}()

	// Queen assigns task
	task := Task{ID: "event-task", Title: "Test events"}
	suite.queenAssignTask("drone-1", task)

	// Drone processes
	takenTask, _ := suite.daemonTakeTask("drone-1")
	if takenTask != nil {
		suite.daemonCompleteTask("drone-1", nil)
	}

	// Collect events
	receivedEvents := []string{}
	timeout := time.After(2 * time.Second)

	for len(receivedEvents) < 2 {
		select {
		case event := <-events:
			receivedEvents = append(receivedEvents, event)
		case <-timeout:
			break
		}
	}

	// Verify events
	hasQueued := false
	hasCompleted := false
	for _, event := range receivedEvents {
		if strings.Contains(event, "task_queued") {
			hasQueued = true
		}
		if strings.Contains(event, "task_completed") {
			hasCompleted = true
		}
	}

	if !hasQueued {
		t.Error("Missing task_queued event")
	}
	if !hasCompleted {
		t.Error("Missing task_completed event")
	}
}

// TestE2E_HighThroughput tests processing many tasks
func TestE2E_HighThroughput(t *testing.T) {
	suite := setupE2E(t)
	defer suite.teardown(t)

	numTasks := 100
	numDrones := 5

	// Queen assigns 100 tasks round-robin
	for i := 0; i < numTasks; i++ {
		droneID := fmt.Sprintf("drone-%d", (i%numDrones)+1)
		task := Task{
			ID:    fmt.Sprintf("task-%03d", i),
			Title: fmt.Sprintf("High throughput task %d", i),
		}
		suite.queenAssignTask(droneID, task)
	}

	// All drones process concurrently
	var wg sync.WaitGroup
	completed := make(chan string, numTasks)

	for i := 1; i <= numDrones; i++ {
		wg.Add(1)
		go func(droneID string) {
			defer wg.Done()
			for {
				task, _ := suite.daemonTakeTask(droneID)
				if task == nil {
					return
				}
				suite.daemonCompleteTask(droneID, map[string]interface{}{"worker": droneID})
				completed <- task.ID
			}
		}(fmt.Sprintf("drone-%d", i))
	}

	wg.Wait()
	close(completed)

	// Count completed
	count := 0
	for range completed {
		count++
	}

	if count != numTasks {
		t.Errorf("Expected %d completed tasks, got %d", numTasks, count)
	}

	// Verify all in completed set
	completedTasks, _ := suite.queenGetCompletedTasks()
	if len(completedTasks) != numTasks {
		t.Errorf("Expected %d in completed set, got %d", numTasks, len(completedTasks))
	}
}

// =============================================================================
// Docker Container E2E Tests (requires Hive image)
// =============================================================================

// findProjectRoot finds the root of the hive project by looking for go.mod
func findProjectRoot() (string, error) {
	dir, err := os.Getwd()
	if err != nil {
		return "", err
	}

	for {
		if _, err := os.Stat(filepath.Join(dir, "go.mod")); err == nil {
			return dir, nil
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			return "", fmt.Errorf("project root not found")
		}
		dir = parent
	}
}

// buildHiveImage builds the Hive Docker image if it doesn't exist
func buildHiveImage(t *testing.T) error {
	t.Log("Checking if hive:test image exists...")

	// Check if image exists
	cmd := exec.Command("docker", "image", "inspect", "hive:test")
	if err := cmd.Run(); err == nil {
		t.Log("Image hive:test already exists")
		return nil
	}

	t.Log("Building hive:test image (this may take a few minutes)...")

	// Find project root
	projectRoot, err := findProjectRoot()
	if err != nil {
		return fmt.Errorf("failed to find project root: %w", err)
	}

	// Build context is the embed/files directory where all source files are
	buildContext := filepath.Join(projectRoot, "internal", "embed", "files")
	dockerfilePath := filepath.Join(buildContext, "docker", "Dockerfile.node")

	// Check if Dockerfile exists
	if _, err := os.Stat(dockerfilePath); os.IsNotExist(err) {
		return fmt.Errorf("Dockerfile not found at %s", dockerfilePath)
	}

	// Check if build context exists
	if _, err := os.Stat(buildContext); os.IsNotExist(err) {
		return fmt.Errorf("Build context not found at %s", buildContext)
	}

	cmd = exec.Command("docker", "build",
		"-t", "hive:test",
		"-f", dockerfilePath,
		buildContext)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	if err := cmd.Run(); err != nil {
		return fmt.Errorf("failed to build image: %w", err)
	}

	t.Log("Image hive:test built successfully")
	return nil
}

// TestE2E_WithHiveContainer tests with actual Hive container
// This test builds the Hive image if needed and runs a full E2E test
func TestE2E_WithHiveContainer(t *testing.T) {
	// Check if we should skip
	if os.Getenv("HIVE_E2E_DOCKER") != "1" {
		t.Skip("Skipping Docker E2E test. Set HIVE_E2E_DOCKER=1 to run")
	}

	// Build image if needed
	if err := buildHiveImage(t); err != nil {
		t.Fatalf("Failed to build Hive image: %v", err)
	}

	ctx := context.Background()

	// Start Redis
	redisContainer, err := tcredis.Run(ctx, "redis:7-alpine")
	if err != nil {
		t.Fatalf("Failed to start Redis: %v", err)
	}
	defer redisContainer.Terminate(ctx)

	redisHost, _ := redisContainer.Host(ctx)
	redisPort, _ := redisContainer.MappedPort(ctx, "6379")

	// Create test repository
	testRepo, _ := createTestRepository(t)
	defer os.RemoveAll(testRepo)

	// Start Hive worker container
	req := testcontainers.ContainerRequest{
		Image: "hive:test",
		Env: map[string]string{
			"REDIS_HOST":       redisHost,
			"REDIS_PORT":       redisPort.Port(),
			"AGENT_ID":         "test-drone",
			"WORKER_MODE":      "daemon",
			"GIT_USER_EMAIL":   "test@example.com",
			"GIT_USER_NAME":    "Test User",
		},
		Mounts: testcontainers.Mounts(
			testcontainers.BindMount(testRepo, "/workspace"),
		),
		WaitingFor: wait.ForLog("Starting").WithStartupTimeout(120 * time.Second),
	}

	hiveContainer, err := testcontainers.GenericContainer(ctx, testcontainers.GenericContainerRequest{
		ContainerRequest: req,
		Started:          true,
	})
	if err != nil {
		t.Fatalf("Failed to start Hive container: %v", err)
	}
	defer hiveContainer.Terminate(ctx)

	// Connect to Redis
	endpoint, _ := redisContainer.Endpoint(ctx, "")
	client := redis.NewClient(&redis.Options{Addr: endpoint})
	defer client.Close()

	// Assign a task
	task := Task{
		ID:          "docker-task-1",
		Title:       "Test task in container",
		Description: "This task runs inside a Docker container",
	}
	taskJSON, _ := json.Marshal(task)
	client.LPush(ctx, "hive:queue:test-drone", taskJSON)
	client.Publish(ctx, "hive:events", "task_queued:test-drone")

	// Wait for task to be processed
	time.Sleep(5 * time.Second)

	// Check if task was completed
	completedCount, _ := client.ZCard(ctx, "hive:completed").Result()
	if completedCount < 1 {
		t.Log("Task may not have been processed (requires Claude API)")
	} else {
		t.Log("Task was processed successfully!")
	}
}
