package cmd

import (
	"bytes"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"time"

	"github.com/mbourmaud/hive/internal/config"
	"github.com/mbourmaud/hive/internal/preflight"
	"github.com/spf13/cobra"
)

var (
	startSkipChecks bool
	startWaitReady  bool
)

var startCmd = &cobra.Command{
	Use:   "start [count]",
	Short: "Start queen + N workers",
	Long:  "Start the hive with queen and specified number of workers (default: 2)",
	Args:  cobra.MaximumNArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		// Load config (from hive.yaml or defaults)
		cfg := config.LoadOrDefault()

		// Run preflight checks unless skipped
		if !startSkipChecks {
			results := preflight.RunAllChecks()
			if !preflight.PrintResults(results) {
				return fmt.Errorf("preflight checks failed. Use --skip-checks to bypass")
			}
		}

		// CLI argument takes precedence over config
		count := cfg.Agents.Workers.Count
		if len(args) > 0 {
			var err error
			count, err = strconv.Atoi(args[0])
			if err != nil {
				return fmt.Errorf("invalid count: %s", args[0])
			}
		}

		if count > 10 {
			return fmt.Errorf("maximum 10 workers allowed")
		}

		if count < 1 {
			return fmt.Errorf("minimum 1 worker required")
		}

		// Create history files to prevent Docker from creating them as directories
		// Docker bind mounts create directories when the source doesn't exist
		hiveDir := ".hive"
		workspacesDir := filepath.Join(hiveDir, "workspaces")
		agents := []string{"queen"}
		for i := 1; i <= count; i++ {
			agents = append(agents, fmt.Sprintf("drone-%d", i))
		}
		for _, agent := range agents {
			// Create workspaces/<agent>/ with history.jsonl and session-env/
			// Matches docker-compose volume mounts: ./workspaces/<agent>/history.jsonl
			agentDir := filepath.Join(workspacesDir, agent)
			sessionEnvDir := filepath.Join(agentDir, "session-env")
			if err := os.MkdirAll(sessionEnvDir, 0755); err != nil {
				return fmt.Errorf("failed to create workspace dir for %s: %w", agent, err)
			}
			historyFile := filepath.Join(agentDir, "history.jsonl")
			if _, err := os.Stat(historyFile); os.IsNotExist(err) {
				if err := os.WriteFile(historyFile, []byte{}, 0644); err != nil {
					return fmt.Errorf("failed to create history file for %s: %w", agent, err)
				}
			}
		}

		fmt.Printf("\n%s%s🚀 Starting Hive%s\n", colorBold, colorCyan, colorReset)
		fmt.Printf("%sQueen + %d worker%s%s\n\n", colorDim, count, pluralize(count), colorReset)

		// Build services list (Redis must start first)
		services := []string{"redis", "queen"}
		for i := 1; i <= count; i++ {
			services = append(services, fmt.Sprintf("agent-%d", i))
		}

		// Start docker compose services from .hive directory
		// Use relative path since we set Dir to hiveDir
		cmdArgs := append([]string{"compose", "-f", "docker-compose.yml", "up", "-d"}, services...)
		dockerCmd := exec.Command("docker", cmdArgs...)

		var stdout, stderr bytes.Buffer
		dockerCmd.Stdout = &stdout
		dockerCmd.Stderr = &stderr
		dockerCmd.Dir = hiveDir

		if err := dockerCmd.Run(); err != nil {
			// Show error in orange box
			printErrorBox("Docker Compose Error", stderr.String())
			return fmt.Errorf("failed to start containers")
		}

		// Show output if any
		if output := stdout.String(); output != "" {
			fmt.Print(output)
		}

		// Wait for containers to be healthy if requested
		if startWaitReady {
			fmt.Printf("%s⏳ Waiting for containers...%s\n", colorCyan, colorReset)
			if err := waitForContainersReady(services, 60*time.Second); err != nil {
				return err
			}
			fmt.Println()
		}

		fmt.Printf("%s%s✨ Hive started successfully!%s\n", colorBold, colorGreen, colorReset)
		fmt.Printf("%s%d container%s running%s\n\n", colorDim, len(services), pluralize(len(services)), colorReset)
		fmt.Printf("%sNext steps:%s\n", colorBold, colorReset)
		fmt.Printf("  %shive connect queen%s  # Connect to orchestrator\n", colorCyan, colorReset)
		fmt.Printf("  %shive connect 1%s      # Connect to worker 1\n", colorCyan, colorReset)
		fmt.Printf("  %shive status%s         # Check status\n\n", colorCyan, colorReset)
		return nil
	},
}

// waitForContainersReady waits for all containers to be running
func waitForContainersReady(services []string, timeout time.Duration) error {
	deadline := time.Now().Add(timeout)

	for _, service := range services {
		var containerName string
		switch service {
		case "queen":
			containerName = "claude-queen"
		case "redis":
			containerName = "hive-redis"
		default:
			containerName = "claude-" + service
		}

		fmt.Printf("  %s%s%s...", colorDim, containerName, colorReset)

		for {
			if time.Now().After(deadline) {
				fmt.Printf(" %sTIMEOUT%s\n", colorYellow, colorReset)
				return fmt.Errorf("timeout waiting for %s to be ready", containerName)
			}

			cmd := exec.Command("docker", "inspect", "-f", "{{.State.Running}}", containerName)
			output, err := cmd.Output()
			if err == nil && string(output) == "true\n" {
				fmt.Printf(" %s✓%s\n", colorGreen, colorReset)
				break
			}

			time.Sleep(1 * time.Second)
		}
	}

	return nil
}

// printErrorBox displays an error message in an orange bordered box
func printErrorBox(title, message string) {
	const (
		colorOrange = "\033[33m"
		colorReset  = "\033[0m"
		colorBold   = "\033[1m"
	)

	// Prepare lines
	lines := strings.Split(strings.TrimSpace(message), "\n")
	maxWidth := len(title)
	for _, line := range lines {
		if len(line) > maxWidth {
			maxWidth = len(line)
		}
	}
	if maxWidth > 80 {
		maxWidth = 80
	}

	// Print box
	fmt.Println()
	fmt.Printf("%s%s╭─ %s ─", colorBold, colorOrange, title)
	for i := 0; i < maxWidth-len(title)-3; i++ {
		fmt.Print("─")
	}
	fmt.Printf("╮%s\n", colorReset)

	// Print content
	for _, line := range lines {
		if len(line) > maxWidth {
			line = line[:maxWidth-3] + "..."
		}
		padding := maxWidth - len(line)
		fmt.Printf("%s%s│%s %s", colorBold, colorOrange, colorReset, line)
		for i := 0; i < padding; i++ {
			fmt.Print(" ")
		}
		fmt.Printf(" %s%s│%s\n", colorBold, colorOrange, colorReset)
	}

	// Print bottom
	fmt.Printf("%s%s╰", colorBold, colorOrange)
	for i := 0; i < maxWidth+2; i++ {
		fmt.Print("─")
	}
	fmt.Printf("╯%s\n\n", colorReset)
}

func init() {
	rootCmd.AddCommand(startCmd)
	startCmd.Flags().BoolVar(&startSkipChecks, "skip-checks", false, "Skip preflight checks")
	startCmd.Flags().BoolVar(&startWaitReady, "wait", false, "Wait for containers to be ready")
}
