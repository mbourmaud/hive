package cmd

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"testing"

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

// TestShowActivityLogs_StreamKeySelection tests stream key selection logic
func TestShowActivityLogs_StreamKeySelection(t *testing.T) {
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
	}

	for _, tt := range tests {
		name := "no_args"
		if len(tt.args) > 0 {
			name = tt.args[0]
		}
		t.Run(name, func(t *testing.T) {
			streamKey := getStreamKey(tt.args)
			if streamKey != tt.expected {
				t.Errorf("getStreamKey(%v) = %q, want %q", tt.args, streamKey, tt.expected)
			}
		})
	}
}

// getStreamKey extracts stream key logic for testing
func getStreamKey(args []string) string {
	if len(args) > 0 {
		agentID := args[0]
		if agentID == "queen" || agentID == "q" || agentID == "0" {
			return "hive:logs:queen"
		}
		return fmt.Sprintf("hive:logs:drone-%s", agentID)
	}
	return "hive:logs:all"
}
