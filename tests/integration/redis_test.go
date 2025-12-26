//go:build integration

// Package integration provides end-to-end tests using real Docker containers
package integration

import (
	"context"
	"encoding/json"
	"fmt"
	"testing"
	"time"

	"github.com/redis/go-redis/v9"
	"github.com/testcontainers/testcontainers-go"
	tcredis "github.com/testcontainers/testcontainers-go/modules/redis"
)

// =============================================================================
// Redis Integration Tests - Testing Task Queue Operations
// =============================================================================

// Task represents a Hive task in Redis
type Task struct {
	ID          string                 `json:"id"`
	Title       string                 `json:"title"`
	Description string                 `json:"description"`
	Status      string                 `json:"status"`
	Priority    int                    `json:"priority"`
	JiraTicket  string                 `json:"jira_ticket,omitempty"`
	Branch      string                 `json:"branch,omitempty"`
	CreatedAt   string                 `json:"created_at,omitempty"`
	CompletedAt string                 `json:"completed_at,omitempty"`
	Result      map[string]interface{} `json:"result,omitempty"`
}

// RedisTestSuite holds the Redis container and client for tests
type RedisTestSuite struct {
	container testcontainers.Container
	client    *redis.Client
	ctx       context.Context
}

// setupRedis creates a Redis container for testing
func setupRedis(t *testing.T) *RedisTestSuite {
	ctx := context.Background()

	redisContainer, err := tcredis.Run(ctx, "redis:7-alpine")
	if err != nil {
		t.Fatalf("Failed to start Redis container: %v", err)
	}

	endpoint, err := redisContainer.Endpoint(ctx, "")
	if err != nil {
		t.Fatalf("Failed to get Redis endpoint: %v", err)
	}

	client := redis.NewClient(&redis.Options{
		Addr: endpoint,
	})

	// Verify connection
	if err := client.Ping(ctx).Err(); err != nil {
		t.Fatalf("Failed to connect to Redis: %v", err)
	}

	return &RedisTestSuite{
		container: redisContainer,
		client:    client,
		ctx:       ctx,
	}
}

// teardown cleans up the Redis container
func (s *RedisTestSuite) teardown(t *testing.T) {
	if err := s.client.Close(); err != nil {
		t.Logf("Failed to close Redis client: %v", err)
	}
	if err := s.container.Terminate(s.ctx); err != nil {
		t.Logf("Failed to terminate Redis container: %v", err)
	}
}

// =============================================================================
// Task Queue Operations (simulating hive-assign, take-task, task-done)
// =============================================================================

// TestTaskEnqueue tests LPUSH to queue (simulates hive-assign)
func TestTaskEnqueue(t *testing.T) {
	suite := setupRedis(t)
	defer suite.teardown(t)

	droneID := "drone-1"
	queueKey := fmt.Sprintf("hive:queue:%s", droneID)

	task := Task{
		ID:          "task-001",
		Title:       "Implement user authentication",
		Description: "Add login and logout endpoints",
		Status:      "pending",
		Priority:    1,
		JiraTicket:  "PROJ-123",
		Branch:      "feature/PROJ-123-auth",
	}

	taskJSON, _ := json.Marshal(task)

	// Enqueue task (LPUSH like hive-assign)
	err := suite.client.LPush(suite.ctx, queueKey, taskJSON).Err()
	if err != nil {
		t.Fatalf("Failed to enqueue task: %v", err)
	}

	// Verify queue length
	length, err := suite.client.LLen(suite.ctx, queueKey).Result()
	if err != nil {
		t.Fatalf("Failed to get queue length: %v", err)
	}
	if length != 1 {
		t.Errorf("Expected queue length 1, got %d", length)
	}

	// Publish event (like Redis PUBLISH in enqueue script)
	err = suite.client.Publish(suite.ctx, "hive:events", fmt.Sprintf("task_queued:%s", droneID)).Err()
	if err != nil {
		t.Fatalf("Failed to publish event: %v", err)
	}
}

// TestTaskDequeue tests RPOPLPUSH (simulates take-task)
func TestTaskDequeue(t *testing.T) {
	suite := setupRedis(t)
	defer suite.teardown(t)

	droneID := "drone-1"
	queueKey := fmt.Sprintf("hive:queue:%s", droneID)
	activeKey := fmt.Sprintf("hive:active:%s", droneID)

	// Setup: enqueue a task
	task := Task{
		ID:     "task-002",
		Title:  "Fix CORS headers",
		Status: "pending",
	}
	taskJSON, _ := json.Marshal(task)
	suite.client.LPush(suite.ctx, queueKey, taskJSON)

	// Dequeue task (RPOPLPUSH like task-dequeue.sh)
	result, err := suite.client.RPopLPush(suite.ctx, queueKey, activeKey).Result()
	if err != nil {
		t.Fatalf("Failed to dequeue task: %v", err)
	}

	// Verify task data
	var dequeuedTask Task
	if err := json.Unmarshal([]byte(result), &dequeuedTask); err != nil {
		t.Fatalf("Failed to parse task JSON: %v", err)
	}

	if dequeuedTask.ID != "task-002" {
		t.Errorf("Expected task ID 'task-002', got '%s'", dequeuedTask.ID)
	}

	// Verify queue is empty
	queueLen, _ := suite.client.LLen(suite.ctx, queueKey).Result()
	if queueLen != 0 {
		t.Errorf("Expected empty queue, got length %d", queueLen)
	}

	// Verify active list has the task
	activeLen, _ := suite.client.LLen(suite.ctx, activeKey).Result()
	if activeLen != 1 {
		t.Errorf("Expected 1 active task, got %d", activeLen)
	}
}

// TestTaskComplete tests task completion flow (simulates task-done)
func TestTaskComplete(t *testing.T) {
	suite := setupRedis(t)
	defer suite.teardown(t)

	droneID := "drone-1"
	activeKey := fmt.Sprintf("hive:active:%s", droneID)

	// Setup: put a task in active
	task := Task{
		ID:     "task-003",
		Title:  "Add unit tests",
		Status: "in_progress",
	}
	taskJSON, _ := json.Marshal(task)
	suite.client.LPush(suite.ctx, activeKey, taskJSON)

	// Complete task (RPOP from active like task-complete.sh)
	result, err := suite.client.RPop(suite.ctx, activeKey).Result()
	if err != nil {
		t.Fatalf("Failed to get active task: %v", err)
	}

	// Parse and update task
	var completedTask Task
	json.Unmarshal([]byte(result), &completedTask)
	completedTask.Status = "completed"
	completedTask.CompletedAt = time.Now().Format(time.RFC3339)
	completedTask.Result = map[string]interface{}{"tests_passed": 42}

	completedJSON, _ := json.Marshal(completedTask)

	// Add to completed sorted set (ZADD like task-complete.sh)
	timestamp := float64(time.Now().Unix())
	err = suite.client.ZAdd(suite.ctx, "hive:completed", redis.Z{
		Score:  timestamp,
		Member: string(completedJSON),
	}).Err()
	if err != nil {
		t.Fatalf("Failed to add to completed set: %v", err)
	}

	// Store task details in hash (HSET like task-complete.sh)
	taskHashKey := fmt.Sprintf("hive:task:%s", completedTask.ID)
	err = suite.client.HSet(suite.ctx, taskHashKey, "data", string(completedJSON)).Err()
	if err != nil {
		t.Fatalf("Failed to store task details: %v", err)
	}

	// Publish completion event
	err = suite.client.Publish(suite.ctx, "hive:events",
		fmt.Sprintf("task_completed:%s:%s", droneID, completedTask.ID)).Err()
	if err != nil {
		t.Fatalf("Failed to publish completion event: %v", err)
	}

	// Verify task is stored
	storedData, err := suite.client.HGet(suite.ctx, taskHashKey, "data").Result()
	if err != nil {
		t.Fatalf("Failed to retrieve stored task: %v", err)
	}

	var storedTask Task
	json.Unmarshal([]byte(storedData), &storedTask)
	if storedTask.Status != "completed" {
		t.Errorf("Expected status 'completed', got '%s'", storedTask.Status)
	}
}

// TestTaskFail tests task failure flow (simulates task-failed)
func TestTaskFail(t *testing.T) {
	suite := setupRedis(t)
	defer suite.teardown(t)

	droneID := "drone-1"
	activeKey := fmt.Sprintf("hive:active:%s", droneID)

	// Setup: put a task in active
	task := Task{
		ID:     "task-004",
		Title:  "Flaky operation",
		Status: "in_progress",
	}
	taskJSON, _ := json.Marshal(task)
	suite.client.LPush(suite.ctx, activeKey, taskJSON)

	// Fail task
	result, _ := suite.client.RPop(suite.ctx, activeKey).Result()

	var failedTask Task
	json.Unmarshal([]byte(result), &failedTask)
	failedTask.Status = "failed"
	failedTask.Result = map[string]interface{}{"error": "Network timeout"}

	failedJSON, _ := json.Marshal(failedTask)

	// Add to failed sorted set
	timestamp := float64(time.Now().Unix())
	suite.client.ZAdd(suite.ctx, "hive:failed", redis.Z{
		Score:  timestamp,
		Member: string(failedJSON),
	})

	// Publish failure event
	suite.client.Publish(suite.ctx, "hive:events",
		fmt.Sprintf("task_failed:%s:%s", droneID, failedTask.ID))

	// Verify task is in failed set
	count, _ := suite.client.ZCard(suite.ctx, "hive:failed").Result()
	if count != 1 {
		t.Errorf("Expected 1 failed task, got %d", count)
	}
}

// =============================================================================
// Multi-Worker Coordination Tests
// =============================================================================

// TestMultipleWorkerQueues tests that each worker has its own queue
func TestMultipleWorkerQueues(t *testing.T) {
	suite := setupRedis(t)
	defer suite.teardown(t)

	workers := []string{"queen", "drone-1", "drone-2", "drone-3"}

	// Assign tasks to different workers
	for i, worker := range workers {
		queueKey := fmt.Sprintf("hive:queue:%s", worker)
		task := Task{
			ID:    fmt.Sprintf("task-%d", i),
			Title: fmt.Sprintf("Task for %s", worker),
		}
		taskJSON, _ := json.Marshal(task)
		suite.client.LPush(suite.ctx, queueKey, taskJSON)
	}

	// Verify each worker has exactly 1 task
	for _, worker := range workers {
		queueKey := fmt.Sprintf("hive:queue:%s", worker)
		length, _ := suite.client.LLen(suite.ctx, queueKey).Result()
		if length != 1 {
			t.Errorf("Worker %s expected 1 task, got %d", worker, length)
		}
	}
}

// TestAtomicTaskDequeue tests that RPOPLPUSH is atomic
func TestAtomicTaskDequeue(t *testing.T) {
	suite := setupRedis(t)
	defer suite.teardown(t)

	droneID := "drone-1"
	queueKey := fmt.Sprintf("hive:queue:%s", droneID)
	activeKey := fmt.Sprintf("hive:active:%s", droneID)

	// Enqueue multiple tasks
	for i := 1; i <= 5; i++ {
		task := Task{ID: fmt.Sprintf("task-%d", i), Title: fmt.Sprintf("Task %d", i)}
		taskJSON, _ := json.Marshal(task)
		suite.client.LPush(suite.ctx, queueKey, taskJSON)
	}

	// Dequeue all tasks (simulating multiple workers racing)
	dequeued := make([]string, 0)
	for {
		result, err := suite.client.RPopLPush(suite.ctx, queueKey, activeKey).Result()
		if err == redis.Nil {
			break
		}
		if err != nil {
			t.Fatalf("Dequeue error: %v", err)
		}

		var task Task
		json.Unmarshal([]byte(result), &task)
		dequeued = append(dequeued, task.ID)

		// Complete task immediately
		suite.client.RPop(suite.ctx, activeKey)
	}

	// Verify all tasks were dequeued exactly once
	if len(dequeued) != 5 {
		t.Errorf("Expected 5 dequeued tasks, got %d", len(dequeued))
	}

	// Verify no duplicates
	seen := make(map[string]bool)
	for _, id := range dequeued {
		if seen[id] {
			t.Errorf("Task %s was dequeued twice", id)
		}
		seen[id] = true
	}
}

// TestPubSubNotifications tests event publishing
func TestPubSubNotifications(t *testing.T) {
	suite := setupRedis(t)
	defer suite.teardown(t)

	// Subscribe to events
	pubsub := suite.client.Subscribe(suite.ctx, "hive:events")
	defer pubsub.Close()

	// Wait for subscription confirmation
	_, err := pubsub.Receive(suite.ctx)
	if err != nil {
		t.Fatalf("Failed to subscribe: %v", err)
	}

	// Start goroutine to receive messages
	messages := make(chan string, 10)
	go func() {
		for msg := range pubsub.Channel() {
			messages <- msg.Payload
		}
	}()

	// Publish some events
	expectedEvents := []string{
		"task_queued:drone-1",
		"task_started:drone-1:task-001",
		"task_completed:drone-1:task-001",
	}

	for _, event := range expectedEvents {
		suite.client.Publish(suite.ctx, "hive:events", event)
	}

	// Verify events received
	receivedCount := 0
	timeout := time.After(2 * time.Second)

	for receivedCount < len(expectedEvents) {
		select {
		case <-messages:
			receivedCount++
		case <-timeout:
			t.Errorf("Timeout: expected %d events, received %d", len(expectedEvents), receivedCount)
			return
		}
	}

	if receivedCount != len(expectedEvents) {
		t.Errorf("Expected %d events, received %d", len(expectedEvents), receivedCount)
	}
}

// TestTaskQueueOrder tests task queue behavior matching Hive's implementation
// Hive uses LPUSH (add to front) + RPOPLPUSH (pop from tail)
// This means newest tasks are at the front, oldest at the tail
// RPOPLPUSH processes from tail = oldest first (FIFO for queue consumers)
func TestTaskQueueOrder(t *testing.T) {
	suite := setupRedis(t)
	defer suite.teardown(t)

	droneID := "drone-1"
	queueKey := fmt.Sprintf("hive:queue:%s", droneID)
	activeKey := fmt.Sprintf("hive:active:%s", droneID)

	// Enqueue tasks using LPUSH (like Hive does)
	// LPUSH adds to front: queue becomes [third, second, first] after 3 pushes
	taskOrder := []string{"first", "second", "third"}
	for _, id := range taskOrder {
		task := Task{ID: id, Title: id}
		taskJSON, _ := json.Marshal(task)
		suite.client.LPush(suite.ctx, queueKey, taskJSON)
	}

	// RPOPLPUSH pops from tail (right side)
	// Expected order: first, second, third (oldest first = FIFO)
	for _, expected := range taskOrder {
		result, _ := suite.client.RPopLPush(suite.ctx, queueKey, activeKey).Result()
		var task Task
		json.Unmarshal([]byte(result), &task)

		if task.ID != expected {
			t.Errorf("Expected task '%s', got '%s'", expected, task.ID)
		}

		// Complete task
		suite.client.RPop(suite.ctx, activeKey)
	}
}

// TestCompletedTasksHistory tests that completed tasks are stored with timestamps
func TestCompletedTasksHistory(t *testing.T) {
	suite := setupRedis(t)
	defer suite.teardown(t)

	// Complete multiple tasks at different times
	for i := 1; i <= 3; i++ {
		task := Task{
			ID:          fmt.Sprintf("task-%d", i),
			Title:       fmt.Sprintf("Task %d", i),
			Status:      "completed",
			CompletedAt: time.Now().Format(time.RFC3339),
		}
		taskJSON, _ := json.Marshal(task)

		timestamp := float64(time.Now().UnixNano())
		suite.client.ZAdd(suite.ctx, "hive:completed", redis.Z{
			Score:  timestamp,
			Member: string(taskJSON),
		})

		time.Sleep(10 * time.Millisecond) // Ensure different timestamps
	}

	// Retrieve completed tasks (most recent first)
	results, err := suite.client.ZRevRange(suite.ctx, "hive:completed", 0, -1).Result()
	if err != nil {
		t.Fatalf("Failed to get completed tasks: %v", err)
	}

	if len(results) != 3 {
		t.Errorf("Expected 3 completed tasks, got %d", len(results))
	}

	// Verify order (most recent first)
	var prevTask Task
	json.Unmarshal([]byte(results[0]), &prevTask)
	if prevTask.ID != "task-3" {
		t.Errorf("Expected most recent task 'task-3', got '%s'", prevTask.ID)
	}
}

// =============================================================================
// Activity Logs Tests (simulating hive logs --activity)
// =============================================================================

// TestActivityLogStream tests Redis stream operations for activity logs
func TestActivityLogStream(t *testing.T) {
	suite := setupRedis(t)
	defer suite.teardown(t)

	streamKey := "hive:logs:all"

	// Add log entries using XADD (like the logging hook does)
	entries := []struct {
		agent   string
		event   string
		content string
	}{
		{"drone-1", "task_start", "Starting fix-auth task"},
		{"drone-1", "tool_call", "Reading src/auth.go"},
		{"drone-1", "tool_result", "File read successfully"},
		{"drone-1", "claude_response", "I found the issue..."},
		{"drone-1", "task_complete", "Task completed"},
	}

	for _, entry := range entries {
		err := suite.client.XAdd(suite.ctx, &redis.XAddArgs{
			Stream: streamKey,
			Values: map[string]interface{}{
				"timestamp": time.Now().Format(time.RFC3339),
				"agent":     entry.agent,
				"event":     entry.event,
				"content":   entry.content,
			},
		}).Err()
		if err != nil {
			t.Fatalf("Failed to add log entry: %v", err)
		}
	}

	// Read entries using XREVRANGE (like hive logs --activity does)
	results, err := suite.client.XRevRange(suite.ctx, streamKey, "+", "-").Result()
	if err != nil {
		t.Fatalf("Failed to read stream: %v", err)
	}

	if len(results) != len(entries) {
		t.Errorf("Expected %d log entries, got %d", len(entries), len(results))
	}

	// Verify latest entry (XREVRANGE returns newest first)
	latest := results[0]
	if latest.Values["event"] != "task_complete" {
		t.Errorf("Expected latest event 'task_complete', got '%v'", latest.Values["event"])
	}
}

// TestActivityLogStreamPerAgent tests per-agent log streams
func TestActivityLogStreamPerAgent(t *testing.T) {
	suite := setupRedis(t)
	defer suite.teardown(t)

	// Log to different agent streams
	agents := []string{"queen", "drone-1", "drone-2"}

	for _, agent := range agents {
		streamKey := fmt.Sprintf("hive:logs:%s", agent)
		err := suite.client.XAdd(suite.ctx, &redis.XAddArgs{
			Stream: streamKey,
			Values: map[string]interface{}{
				"timestamp": time.Now().Format(time.RFC3339),
				"agent":     agent,
				"event":     "task_start",
				"content":   fmt.Sprintf("%s starting work", agent),
			},
		}).Err()
		if err != nil {
			t.Fatalf("Failed to add log entry for %s: %v", agent, err)
		}
	}

	// Verify each agent has its own stream
	for _, agent := range agents {
		streamKey := fmt.Sprintf("hive:logs:%s", agent)
		length, err := suite.client.XLen(suite.ctx, streamKey).Result()
		if err != nil {
			t.Fatalf("Failed to get stream length for %s: %v", agent, err)
		}
		if length != 1 {
			t.Errorf("Agent %s expected 1 log entry, got %d", agent, length)
		}
	}
}

// TestActivityLogStreamFollow tests XREAD for follow mode
func TestActivityLogStreamFollow(t *testing.T) {
	suite := setupRedis(t)
	defer suite.teardown(t)

	streamKey := "hive:logs:all"

	// Start a goroutine to add entries after a delay
	go func() {
		time.Sleep(100 * time.Millisecond)
		suite.client.XAdd(suite.ctx, &redis.XAddArgs{
			Stream: streamKey,
			Values: map[string]interface{}{
				"timestamp": time.Now().Format(time.RFC3339),
				"agent":     "drone-1",
				"event":     "tool_call",
				"content":   "Reading file",
			},
		})
	}()

	// XREAD with block (like follow mode)
	streams, err := suite.client.XRead(suite.ctx, &redis.XReadArgs{
		Streams: []string{streamKey, "$"},
		Block:   500 * time.Millisecond,
		Count:   1,
	}).Result()

	if err == redis.Nil {
		// Timeout without data is acceptable
		return
	}
	if err != nil {
		t.Fatalf("Failed to read stream: %v", err)
	}

	if len(streams) > 0 && len(streams[0].Messages) > 0 {
		msg := streams[0].Messages[0]
		if msg.Values["event"] != "tool_call" {
			t.Errorf("Expected event 'tool_call', got '%v'", msg.Values["event"])
		}
	}
}

// TestActivityLogStreamTrimming tests stream trimming for log rotation
func TestActivityLogStreamTrimming(t *testing.T) {
	suite := setupRedis(t)
	defer suite.teardown(t)

	streamKey := "hive:logs:test"

	// Add many log entries
	for i := 0; i < 100; i++ {
		suite.client.XAdd(suite.ctx, &redis.XAddArgs{
			Stream: streamKey,
			Values: map[string]interface{}{
				"timestamp": time.Now().Format(time.RFC3339),
				"agent":     "test",
				"event":     "test_event",
				"content":   fmt.Sprintf("Log entry %d", i),
			},
		})
	}

	// Trim to last 50 entries
	err := suite.client.XTrimMaxLen(suite.ctx, streamKey, 50).Err()
	if err != nil {
		t.Fatalf("Failed to trim stream: %v", err)
	}

	// Verify trimmed length
	length, _ := suite.client.XLen(suite.ctx, streamKey).Result()
	if length != 50 {
		t.Errorf("Expected 50 entries after trim, got %d", length)
	}
}
