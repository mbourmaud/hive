package cmd

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"

	"github.com/mbourmaud/hive/internal/agent"
	"github.com/mbourmaud/hive/internal/ui"
	"github.com/spf13/cobra"
)

var logsCmd = &cobra.Command{
	Use:   "logs <agent>",
	Short: "Stream logs from an agent in real-time",
	Long: `Stream real-time logs from a running agent using AgentAPI SSE events.

Shows message updates and status changes as they happen.

Examples:
  hive logs frontend
  hive logs backend --no-color`,
	Args: cobra.ExactArgs(1),
	RunE: runLogs,
}

var logsNoColor bool

func init() {
	rootCmd.AddCommand(logsCmd)
	logsCmd.Flags().BoolVar(&logsNoColor, "no-color", false, "Disable colored output")
}

type SSEEvent struct {
	Type string          `json:"type"`
	Data json.RawMessage `json:"data"`
}

type MessageUpdate struct {
	ID      int    `json:"id"`
	Role    string `json:"role"`
	Message string `json:"message"`
	Time    string `json:"time"`
}

type StatusUpdate struct {
	Status string `json:"status"`
}

func runLogs(cmd *cobra.Command, args []string) error {
	agentName := args[0]

	a, err := getAgentFromState(agentName)
	if err != nil {
		return fmt.Errorf("agent not found: %w", err)
	}

	client := agent.NewHTTPClient()
	if !client.Health(cmd.Context(), a.Port) {
		return fmt.Errorf("agent %s is not responding (port %d)", agentName, a.Port)
	}

	fmt.Printf("%s Streaming logs from %s (port %d)...\n",
		ui.StyleCyan.Render("📡"),
		ui.StyleBold.Render(a.Name),
		a.Port)
	fmt.Println(ui.StyleDim.Render("Press Ctrl+C to stop\n"))

	ctx, cancel := context.WithCancel(cmd.Context())
	defer cancel()

	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-sigChan
		cancel()
	}()

	return streamLogs(ctx, a.Port)
}

func streamLogs(ctx context.Context, port int) error {
	url := fmt.Sprintf("http://localhost:%d/events", port)

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}
	req.Header.Set("Accept", "text/event-stream")
	req.Header.Set("Cache-Control", "no-cache")

	client := &http.Client{Timeout: 0}
	resp, err := client.Do(req)
	if err != nil {
		return fmt.Errorf("failed to connect to event stream: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("event stream returned status %d", resp.StatusCode)
	}

	scanner := bufio.NewScanner(resp.Body)
	var eventType string
	var dataLines []string

	for scanner.Scan() {
		select {
		case <-ctx.Done():
			return nil
		default:
		}

		line := scanner.Text()

		if strings.HasPrefix(line, "event:") {
			eventType = strings.TrimSpace(strings.TrimPrefix(line, "event:"))
			continue
		}

		if strings.HasPrefix(line, "data:") {
			dataLines = append(dataLines, strings.TrimPrefix(line, "data:"))
			continue
		}

		if line == "" && len(dataLines) > 0 {
			data := strings.Join(dataLines, "\n")
			handleSSEEvent(eventType, data)
			eventType = ""
			dataLines = nil
		}
	}

	if err := scanner.Err(); err != nil {
		if ctx.Err() != nil {
			return nil
		}
		return fmt.Errorf("error reading event stream: %w", err)
	}

	return nil
}

func handleSSEEvent(eventType, data string) {
	timestamp := time.Now().Format("15:04:05")

	switch eventType {
	case "message":
		var msg MessageUpdate
		if err := json.Unmarshal([]byte(data), &msg); err != nil {
			return
		}

		roleStyle := ui.StyleCyan
		rolePrefix := "🤖"
		if msg.Role == "user" {
			roleStyle = ui.StyleGreen
			rolePrefix = "👤"
		}

		content := msg.Message
		if len(content) > 200 {
			lines := strings.Split(content, "\n")
			if len(lines) > 5 {
				content = strings.Join(lines[:5], "\n") + "\n..."
			}
		}

		if logsNoColor {
			fmt.Printf("[%s] %s %s:\n%s\n\n", timestamp, rolePrefix, msg.Role, content)
		} else {
			fmt.Printf("%s %s %s:\n%s\n\n",
				ui.StyleDim.Render("["+timestamp+"]"),
				rolePrefix,
				roleStyle.Render(msg.Role),
				content)
		}

	case "status":
		var status StatusUpdate
		if err := json.Unmarshal([]byte(data), &status); err != nil {
			return
		}

		statusStyle := ui.StyleYellow
		statusIcon := "⏳"
		if status.Status == "stable" {
			statusStyle = ui.StyleGreen
			statusIcon = "✓"
		}

		if logsNoColor {
			fmt.Printf("[%s] %s Status: %s\n", timestamp, statusIcon, status.Status)
		} else {
			fmt.Printf("%s %s Status: %s\n",
				ui.StyleDim.Render("["+timestamp+"]"),
				statusIcon,
				statusStyle.Render(status.Status))
		}
	}
}
