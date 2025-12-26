package cmd

import (
	"bytes"
	"io"
	"os"
	"strings"
	"testing"

	"github.com/go-redis/redismock/v9"
	"github.com/redis/go-redis/v9"
)

// TestPrintActivityEntry tests the printActivityEntry function
func TestPrintActivityEntry(t *testing.T) {
	tests := []struct {
		name     string
		msg      redis.XMessage
		expected []string // substrings that should appear in output
	}{
		{
			name: "task_start event",
			msg: redis.XMessage{
				ID: "1234567890123-0",
				Values: map[string]interface{}{
					"timestamp": "2024-01-15T10:30:00Z",
					"agent":     "drone-1",
					"event":     "task_start",
					"content":   "Starting task",
				},
			},
			expected: []string{"🚀", "drone-1", "task_start", "Starting task"},
		},
		{
			name: "claude_response event",
			msg: redis.XMessage{
				ID: "1234567890124-0",
				Values: map[string]interface{}{
					"timestamp": "2024-01-15T10:31:00Z",
					"agent":     "queen",
					"event":     "claude_response",
					"content":   "Analyzing code",
				},
			},
			expected: []string{"💬", "queen", "claude_response", "Analyzing code"},
		},
		{
			name: "tool_call event",
			msg: redis.XMessage{
				ID: "1234567890125-0",
				Values: map[string]interface{}{
					"timestamp": "2024-01-15T10:32:00Z",
					"agent":     "drone-2",
					"event":     "tool_call",
					"content":   "Read file main.go",
				},
			},
			expected: []string{"🔧", "drone-2", "tool_call", "Read file main.go"},
		},
		{
			name: "tool_result event",
			msg: redis.XMessage{
				ID: "1234567890126-0",
				Values: map[string]interface{}{
					"timestamp": "2024-01-15T10:33:00Z",
					"agent":     "drone-1",
					"event":     "tool_result",
					"content":   "File read successfully",
				},
			},
			expected: []string{"✓", "drone-1", "tool_result"},
		},
		{
			name: "tool_error event",
			msg: redis.XMessage{
				ID: "1234567890127-0",
				Values: map[string]interface{}{
					"timestamp": "2024-01-15T10:34:00Z",
					"agent":     "drone-1",
					"event":     "tool_error",
					"content":   "File not found",
				},
			},
			expected: []string{"❌", "tool_error", "File not found"},
		},
		{
			name: "task_complete event",
			msg: redis.XMessage{
				ID: "1234567890128-0",
				Values: map[string]interface{}{
					"timestamp": "2024-01-15T10:35:00Z",
					"agent":     "drone-1",
					"event":     "task_complete",
					"content":   "Task completed successfully",
				},
			},
			expected: []string{"✅", "task_complete"},
		},
		{
			name: "task_failed event",
			msg: redis.XMessage{
				ID: "1234567890129-0",
				Values: map[string]interface{}{
					"timestamp": "2024-01-15T10:36:00Z",
					"agent":     "drone-1",
					"event":     "task_failed",
					"content":   "Task failed with error",
				},
			},
			expected: []string{"💥", "task_failed"},
		},
		{
			name: "unknown event",
			msg: redis.XMessage{
				ID: "1234567890130-0",
				Values: map[string]interface{}{
					"timestamp": "2024-01-15T10:37:00Z",
					"agent":     "drone-1",
					"event":     "custom_event",
					"content":   "Custom content",
				},
			},
			expected: []string{"•", "custom_event"},
		},
		{
			name: "long content gets truncated",
			msg: redis.XMessage{
				ID: "1234567890131-0",
				Values: map[string]interface{}{
					"timestamp": "2024-01-15T10:38:00Z",
					"agent":     "drone-1",
					"event":     "claude_response",
					"content":   "This is a very long content that should be truncated because it exceeds the 100 character limit set in the printActivityEntry function to keep the output readable",
				},
			},
			expected: []string{"...", "💬"},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Capture stdout
			old := os.Stdout
			r, w, _ := os.Pipe()
			os.Stdout = w

			printActivityEntry(tt.msg)

			w.Close()
			os.Stdout = old

			var buf bytes.Buffer
			io.Copy(&buf, r)
			output := buf.String()

			for _, exp := range tt.expected {
				if !bytes.Contains([]byte(output), []byte(exp)) {
					t.Errorf("printActivityEntry() output missing %q\nGot: %s", exp, output)
				}
			}
		})
	}
}

// TestGetActivityStreamKey tests stream key selection logic
func TestGetActivityStreamKey(t *testing.T) {
	tests := []struct {
		args     []string
		expected string
	}{
		{[]string{}, "hive:logs:all"},
		{[]string{"queen"}, "hive:logs:queen"},
		{[]string{"q"}, "hive:logs:queen"},
		{[]string{"0"}, "hive:logs:queen"},
		{[]string{"1"}, "hive:logs:drone-1"},
		{[]string{"2"}, "hive:logs:drone-2"},
		{[]string{"10"}, "hive:logs:drone-10"},
	}

	for _, tt := range tests {
		name := "no_args"
		if len(tt.args) > 0 {
			name = tt.args[0]
		}
		t.Run(name, func(t *testing.T) {
			streamKey := getActivityStreamKey(tt.args)
			if streamKey != tt.expected {
				t.Errorf("getActivityStreamKey(%v) = %q, want %q", tt.args, streamKey, tt.expected)
			}
		})
	}
}

// TestShowActivityLogs_WithMock tests showActivityLogs with mocked Redis
func TestShowActivityLogs_WithMock(t *testing.T) {
	// Create mock
	db, mock := redismock.NewClientMock()

	// Set up the mock client
	redisClient = db
	defer func() { redisClient = nil }()

	// Mock XRevRange response
	mock.ExpectXRevRange("hive:logs:all", "+", "-").SetVal([]redis.XMessage{
		{
			ID: "1234567890123-0",
			Values: map[string]interface{}{
				"timestamp": "2024-01-15T10:30:00Z",
				"agent":     "drone-1",
				"event":     "task_start",
				"content":   "Starting task",
			},
		},
		{
			ID: "1234567890122-0",
			Values: map[string]interface{}{
				"timestamp": "2024-01-15T10:29:00Z",
				"agent":     "queen",
				"event":     "claude_response",
				"content":   "Analyzing...",
			},
		},
	})

	// Capture stdout
	old := os.Stdout
	r, w, _ := os.Pipe()
	os.Stdout = w

	// Reset logsFollow and logsTail for this test
	logsFollow = false
	logsTail = 100

	err := showActivityLogs([]string{})

	w.Close()
	os.Stdout = old

	var buf bytes.Buffer
	io.Copy(&buf, r)
	output := buf.String()

	if err != nil {
		t.Fatalf("showActivityLogs() error = %v", err)
	}

	// Verify output contains expected content
	if !strings.Contains(output, "All activity logs") {
		t.Error("showActivityLogs() missing header")
	}
	if !strings.Contains(output, "drone-1") {
		t.Error("showActivityLogs() missing drone-1 entry")
	}

	// Verify mock expectations
	if err := mock.ExpectationsWereMet(); err != nil {
		t.Errorf("mock expectations not met: %v", err)
	}
}

// TestShowActivityLogs_EmptyStream tests empty stream handling
func TestShowActivityLogs_EmptyStream(t *testing.T) {
	db, mock := redismock.NewClientMock()
	redisClient = db
	defer func() { redisClient = nil }()

	// Mock empty response
	mock.ExpectXRevRange("hive:logs:all", "+", "-").SetVal([]redis.XMessage{})

	// Capture stdout
	old := os.Stdout
	r, w, _ := os.Pipe()
	os.Stdout = w

	logsFollow = false
	logsTail = 100

	err := showActivityLogs([]string{})

	w.Close()
	os.Stdout = old

	var buf bytes.Buffer
	io.Copy(&buf, r)
	output := buf.String()

	if err != nil {
		t.Fatalf("showActivityLogs() error = %v", err)
	}

	if !strings.Contains(output, "No activity logs found") {
		t.Error("showActivityLogs() should show 'No activity logs found' for empty stream")
	}

	if err := mock.ExpectationsWereMet(); err != nil {
		t.Errorf("mock expectations not met: %v", err)
	}
}

// TestShowActivityLogs_SpecificAgent tests agent-specific logs
func TestShowActivityLogs_SpecificAgent(t *testing.T) {
	db, mock := redismock.NewClientMock()
	redisClient = db
	defer func() { redisClient = nil }()

	// Mock XRevRange for specific agent
	mock.ExpectXRevRange("hive:logs:drone-1", "+", "-").SetVal([]redis.XMessage{
		{
			ID: "1234567890123-0",
			Values: map[string]interface{}{
				"timestamp": "2024-01-15T10:30:00Z",
				"agent":     "drone-1",
				"event":     "task_start",
				"content":   "Working on task",
			},
		},
	})

	// Capture stdout
	old := os.Stdout
	r, w, _ := os.Pipe()
	os.Stdout = w

	logsFollow = false
	logsTail = 100

	err := showActivityLogs([]string{"1"})

	w.Close()
	os.Stdout = old

	var buf bytes.Buffer
	io.Copy(&buf, r)
	output := buf.String()

	if err != nil {
		t.Fatalf("showActivityLogs() error = %v", err)
	}

	if !strings.Contains(output, "hive:logs:drone-1") {
		t.Error("showActivityLogs() should show specific agent stream key")
	}

	if err := mock.ExpectationsWereMet(); err != nil {
		t.Errorf("mock expectations not met: %v", err)
	}
}

// TestShowActivityLogs_TailLimit tests the tail limit functionality
func TestShowActivityLogs_TailLimit(t *testing.T) {
	db, mock := redismock.NewClientMock()
	redisClient = db
	defer func() { redisClient = nil }()

	// Create 10 messages
	messages := make([]redis.XMessage, 10)
	for i := 0; i < 10; i++ {
		messages[i] = redis.XMessage{
			ID: "123456789012" + string(rune('0'+i)) + "-0",
			Values: map[string]interface{}{
				"timestamp": "2024-01-15T10:30:00Z",
				"agent":     "drone-1",
				"event":     "task_start",
				"content":   "Message",
			},
		}
	}

	mock.ExpectXRevRange("hive:logs:all", "+", "-").SetVal(messages)

	// Capture stdout
	old := os.Stdout
	r, w, _ := os.Pipe()
	os.Stdout = w

	logsFollow = false
	logsTail = 5 // Only show 5 entries

	err := showActivityLogs([]string{})

	w.Close()
	os.Stdout = old

	var buf bytes.Buffer
	io.Copy(&buf, r)

	if err != nil {
		t.Fatalf("showActivityLogs() error = %v", err)
	}

	if err := mock.ExpectationsWereMet(); err != nil {
		t.Errorf("mock expectations not met: %v", err)
	}
}
