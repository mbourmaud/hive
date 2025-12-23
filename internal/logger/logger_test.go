package logger

import (
	"bytes"
	"encoding/json"
	"strings"
	"testing"
)

func TestNew(t *testing.T) {
	l := New()
	if l == nil {
		t.Fatal("New() returned nil")
		return
	}
	if l.level != LevelInfo {
		t.Errorf("expected default level to be INFO, got %s", l.level)
	}
}

func TestSetLevel(t *testing.T) {
	l := New()
	l.SetLevel(LevelDebug)
	if l.level != LevelDebug {
		t.Errorf("expected level to be DEBUG, got %s", l.level)
	}
}

func TestLogLevels(t *testing.T) {
	buf := &bytes.Buffer{}
	l := New()
	l.SetOutput(buf)
	l.SetLevel(LevelDebug)

	l.Debug("debug message")
	l.Info("info message")
	l.Warn("warn message")
	l.Error("error message")

	output := buf.String()
	if !strings.Contains(output, "[DEBUG]") {
		t.Error("expected output to contain [DEBUG]")
	}
	if !strings.Contains(output, "[INFO]") {
		t.Error("expected output to contain [INFO]")
	}
	if !strings.Contains(output, "[WARN]") {
		t.Error("expected output to contain [WARN]")
	}
	if !strings.Contains(output, "[ERROR]") {
		t.Error("expected output to contain [ERROR]")
	}
}

func TestLogFiltering(t *testing.T) {
	buf := &bytes.Buffer{}
	l := New()
	l.SetOutput(buf)
	l.SetLevel(LevelWarn) // Only WARN and ERROR should be logged

	l.Debug("debug message")
	l.Info("info message")
	l.Warn("warn message")
	l.Error("error message")

	output := buf.String()
	if strings.Contains(output, "[DEBUG]") {
		t.Error("DEBUG should be filtered out")
	}
	if strings.Contains(output, "[INFO]") {
		t.Error("INFO should be filtered out")
	}
	if !strings.Contains(output, "[WARN]") {
		t.Error("WARN should be present")
	}
	if !strings.Contains(output, "[ERROR]") {
		t.Error("ERROR should be present")
	}
}

func TestJSONOutput(t *testing.T) {
	buf := &bytes.Buffer{}
	l := New()
	l.SetOutput(buf)
	l.SetJSON(true)

	l.Info("test message")

	var entry Entry
	if err := json.Unmarshal(buf.Bytes(), &entry); err != nil {
		t.Fatalf("failed to parse JSON output: %v", err)
	}

	if entry.Level != "INFO" {
		t.Errorf("expected level to be INFO, got %s", entry.Level)
	}
	if entry.Message != "test message" {
		t.Errorf("expected message to be 'test message', got '%s'", entry.Message)
	}
}

func TestWithField(t *testing.T) {
	buf := &bytes.Buffer{}
	l := New()
	l.SetOutput(buf)

	l2 := l.WithField("key", "value")
	l2.Info("test message")

	output := buf.String()
	if !strings.Contains(output, "key=value") {
		t.Errorf("expected output to contain 'key=value', got: %s", output)
	}
}

func TestWithFields(t *testing.T) {
	buf := &bytes.Buffer{}
	l := New()
	l.SetOutput(buf)

	l2 := l.WithFields(map[string]interface{}{
		"key1": "value1",
		"key2": 42,
	})
	l2.Info("test message")

	output := buf.String()
	if !strings.Contains(output, "key1=value1") {
		t.Errorf("expected output to contain 'key1=value1', got: %s", output)
	}
	if !strings.Contains(output, "key2=42") {
		t.Errorf("expected output to contain 'key2=42', got: %s", output)
	}
}

func TestDisable(t *testing.T) {
	buf := &bytes.Buffer{}
	l := New()
	l.SetOutput(buf)

	l.Disable()
	l.Info("this should not appear")

	if buf.Len() > 0 {
		t.Error("expected no output when logger is disabled")
	}

	l.Enable()
	l.Info("this should appear")

	if buf.Len() == 0 {
		t.Error("expected output when logger is enabled")
	}
}

func TestLevelString(t *testing.T) {
	tests := []struct {
		level    Level
		expected string
	}{
		{LevelDebug, "DEBUG"},
		{LevelInfo, "INFO"},
		{LevelWarn, "WARN"},
		{LevelError, "ERROR"},
		{Level(100), "UNKNOWN"},
	}

	for _, tt := range tests {
		if got := tt.level.String(); got != tt.expected {
			t.Errorf("Level(%d).String() = %s, expected %s", tt.level, got, tt.expected)
		}
	}
}

func TestDefaultLogger(t *testing.T) {
	// Just verify default logger functions don't panic
	SetDefaultLevel(LevelDebug)
	SetDefaultJSON(false)

	// These should not panic
	Debug("debug")
	Info("info")
	Warn("warn")
	Error("error")

	l := WithField("key", "value")
	if l == nil {
		t.Error("WithField returned nil")
	}

	l = WithFields(map[string]interface{}{"key": "value"})
	if l == nil {
		t.Error("WithFields returned nil")
	}
}
