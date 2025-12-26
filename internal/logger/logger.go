package logger

import (
	"encoding/json"
	"fmt"
	"io"
	"maps"
	"os"
	"sync"
	"time"
)

// Level represents the severity of a log message
type Level int

const (
	LevelDebug Level = iota
	LevelInfo
	LevelWarn
	LevelError
)

func (l Level) String() string {
	switch l {
	case LevelDebug:
		return "DEBUG"
	case LevelInfo:
		return "INFO"
	case LevelWarn:
		return "WARN"
	case LevelError:
		return "ERROR"
	default:
		return "UNKNOWN"
	}
}

// Entry represents a log entry
type Entry struct {
	Timestamp string                 `json:"timestamp"`
	Level     string                 `json:"level"`
	Message   string                 `json:"message"`
	Fields    map[string]interface{} `json:"fields,omitempty"`
}

// Logger provides structured logging capabilities
type Logger struct {
	mu       sync.Mutex
	output   io.Writer
	level    Level
	json     bool
	fields   map[string]interface{}
	disabled bool
}

// New creates a new logger with default settings
func New() *Logger {
	return &Logger{
		output: os.Stdout,
		level:  LevelInfo,
		json:   false,
		fields: make(map[string]interface{}),
	}
}

// SetOutput sets the output destination
func (l *Logger) SetOutput(w io.Writer) {
	l.mu.Lock()
	defer l.mu.Unlock()
	l.output = w
}

// SetLevel sets the minimum log level
func (l *Logger) SetLevel(level Level) {
	l.mu.Lock()
	defer l.mu.Unlock()
	l.level = level
}

// SetJSON enables or disables JSON output
func (l *Logger) SetJSON(enabled bool) {
	l.mu.Lock()
	defer l.mu.Unlock()
	l.json = enabled
}

// Disable disables all logging
func (l *Logger) Disable() {
	l.mu.Lock()
	defer l.mu.Unlock()
	l.disabled = true
}

// Enable enables logging
func (l *Logger) Enable() {
	l.mu.Lock()
	defer l.mu.Unlock()
	l.disabled = false
}

// WithField returns a new logger with an additional field
func (l *Logger) WithField(key string, value interface{}) *Logger {
	l.mu.Lock()
	defer l.mu.Unlock()

	// Use maps.Clone for efficient shallow copy (Go 1.21+)
	newFields := maps.Clone(l.fields)
	if newFields == nil {
		newFields = make(map[string]interface{}, 1)
	}
	newFields[key] = value

	return &Logger{
		output:   l.output,
		level:    l.level,
		json:     l.json,
		fields:   newFields,
		disabled: l.disabled,
	}
}

// WithFields returns a new logger with additional fields
func (l *Logger) WithFields(fields map[string]interface{}) *Logger {
	l.mu.Lock()
	defer l.mu.Unlock()

	// Pre-size new map for efficiency
	newFields := make(map[string]interface{}, len(l.fields)+len(fields))
	maps.Copy(newFields, l.fields)
	maps.Copy(newFields, fields)

	return &Logger{
		output:   l.output,
		level:    l.level,
		json:     l.json,
		fields:   newFields,
		disabled: l.disabled,
	}
}

func (l *Logger) log(level Level, format string, args ...interface{}) {
	l.mu.Lock()
	defer l.mu.Unlock()

	if l.disabled || level < l.level {
		return
	}

	message := fmt.Sprintf(format, args...)
	timestamp := time.Now().Format(time.RFC3339)

	if l.json {
		entry := Entry{
			Timestamp: timestamp,
			Level:     level.String(),
			Message:   message,
			Fields:    l.fields,
		}
		data, err := json.Marshal(entry)
		if err != nil {
			_, _ = fmt.Fprintf(l.output, `{"error":"failed to marshal log entry: %s"}`, err)
			return
		}
		_, _ = fmt.Fprintln(l.output, string(data))
	} else {
		// Human-readable format
		prefix := ""
		switch level {
		case LevelDebug:
			prefix = "[DEBUG]"
		case LevelInfo:
			prefix = "[INFO]"
		case LevelWarn:
			prefix = "[WARN]"
		case LevelError:
			prefix = "[ERROR]"
		}

		if len(l.fields) > 0 {
			fieldsStr := ""
			for k, v := range l.fields {
				fieldsStr += fmt.Sprintf(" %s=%v", k, v)
			}
			_, _ = fmt.Fprintf(l.output, "%s %s %s%s\n", timestamp, prefix, message, fieldsStr)
		} else {
			_, _ = fmt.Fprintf(l.output, "%s %s %s\n", timestamp, prefix, message)
		}
	}
}

// Debug logs a debug message
func (l *Logger) Debug(format string, args ...interface{}) {
	l.log(LevelDebug, format, args...)
}

// Info logs an info message
func (l *Logger) Info(format string, args ...interface{}) {
	l.log(LevelInfo, format, args...)
}

// Warn logs a warning message
func (l *Logger) Warn(format string, args ...interface{}) {
	l.log(LevelWarn, format, args...)
}

// Error logs an error message
func (l *Logger) Error(format string, args ...interface{}) {
	l.log(LevelError, format, args...)
}

// Default logger instance
var defaultLogger = New()

// SetDefaultLevel sets the level for the default logger
func SetDefaultLevel(level Level) {
	defaultLogger.SetLevel(level)
}

// SetDefaultJSON enables JSON output for the default logger
func SetDefaultJSON(enabled bool) {
	defaultLogger.SetJSON(enabled)
}

// Debug logs a debug message using the default logger
func Debug(format string, args ...interface{}) {
	defaultLogger.Debug(format, args...)
}

// Info logs an info message using the default logger
func Info(format string, args ...interface{}) {
	defaultLogger.Info(format, args...)
}

// Warn logs a warning message using the default logger
func Warn(format string, args ...interface{}) {
	defaultLogger.Warn(format, args...)
}

// Error logs an error message using the default logger
func Error(format string, args ...interface{}) {
	defaultLogger.Error(format, args...)
}

// WithField returns a new logger with an additional field using the default logger
func WithField(key string, value interface{}) *Logger {
	return defaultLogger.WithField(key, value)
}

// WithFields returns a new logger with additional fields using the default logger
func WithFields(fields map[string]interface{}) *Logger {
	return defaultLogger.WithFields(fields)
}
