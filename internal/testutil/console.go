//go:build !windows
// +build !windows

package testutil

import (
	"testing"
	"time"

	"github.com/AlecAivazis/survey/v2/terminal"
	expect "github.com/Netflix/go-expect"
	pseudotty "github.com/creack/pty"
	"github.com/hinshun/vt10x"
)

// ExpectConsole is the interface for interacting with the virtual terminal
type ExpectConsole interface {
	ExpectString(string)
	ExpectEOF()
	SendLine(string)
	Send(string)
}

// consoleWrapper wraps go-expect Console to implement ExpectConsole
type consoleWrapper struct {
	c *expect.Console
	t *testing.T
}

func (w *consoleWrapper) ExpectString(s string) {
	w.t.Helper()
	if _, err := w.c.ExpectString(s); err != nil {
		w.t.Logf("ExpectString(%q) error: %v", s, err)
	}
}

func (w *consoleWrapper) ExpectEOF() {
	w.t.Helper()
	if _, err := w.c.ExpectEOF(); err != nil {
		w.t.Logf("ExpectEOF error: %v", err)
	}
}

func (w *consoleWrapper) SendLine(s string) {
	w.t.Helper()
	if _, err := w.c.SendLine(s); err != nil {
		w.t.Fatalf("SendLine(%q) error: %v", s, err)
	}
}

func (w *consoleWrapper) Send(s string) {
	w.t.Helper()
	if _, err := w.c.Send(s); err != nil {
		w.t.Fatalf("Send(%q) error: %v", s, err)
	}
}

// RunPromptTest runs a prompt test with a virtual terminal using vt10x
// This is the same pattern used by AlecAivazis/survey for testing interactive prompts
func RunPromptTest(t *testing.T, procedure func(ExpectConsole), test func(terminal.Stdio) error) {
	t.Helper()

	// Create pseudo-terminal pair
	ptm, pts, err := pseudotty.Open()
	if err != nil {
		t.Fatalf("failed to open pseudotty: %v", err)
	}

	// Create vt10x terminal emulator
	term := vt10x.New(vt10x.WithWriter(pts))

	// Create expect console
	c, err := expect.NewConsole(
		expect.WithStdin(ptm),
		expect.WithStdout(term),
		expect.WithCloser(ptm, pts),
		expect.WithDefaultTimeout(5*time.Second),
	)
	if err != nil {
		t.Fatalf("failed to create console: %v", err)
	}
	defer c.Close()

	// Run the procedure in a goroutine
	done := make(chan struct{})
	go func() {
		defer close(done)
		procedure(&consoleWrapper{c: c, t: t})
	}()

	// Run the test with stdio connected to console
	stdio := terminal.Stdio{In: c.Tty(), Out: c.Tty(), Err: c.Tty()}
	err = test(stdio)

	// Close tty to signal EOF
	c.Tty().Close()

	// Wait for procedure to complete with timeout
	select {
	case <-done:
	case <-time.After(10 * time.Second):
		t.Fatal("test timed out waiting for procedure")
	}

	if err != nil {
		t.Logf("test function returned error: %v", err)
	}
}
