package cmd

import (
	"fmt"
	"os/exec"
	"runtime"

	"github.com/mbourmaud/hive/internal/monitor"
	"github.com/spf13/cobra"
)

var monitorCmd = &cobra.Command{
	Use:   "monitor",
	Short: "Monitor Hive agents in real-time",
	Long: `Monitor Hive agents, tasks, and solicitations in real-time.

By default, opens a TUI (Terminal User Interface) dashboard.
Use --web to open a web-based dashboard instead.

Examples:
  hive monitor              # TUI dashboard in terminal
  hive monitor --web        # Web dashboard at http://localhost:3000
  hive monitor --hub :9000  # Connect to hub on custom port`,
	RunE: runMonitor,
}

var (
	monitorWeb     bool
	monitorWebPort int
	monitorHubURL  string
)

func init() {
	rootCmd.AddCommand(monitorCmd)

	monitorCmd.Flags().BoolVar(&monitorWeb, "web", false, "Open web-based dashboard instead of TUI")
	monitorCmd.Flags().IntVar(&monitorWebPort, "port", 3000, "Port for web dashboard")
	monitorCmd.Flags().StringVar(&monitorHubURL, "hub", "http://localhost:8080", "Hub API URL")
}

func runMonitor(cmd *cobra.Command, args []string) error {
	if monitorWeb {
		return runWebMonitor()
	}
	return runTUIMonitor()
}

func runTUIMonitor() error {
	return monitor.RunTUI(monitorHubURL)
}

func runWebMonitor() error {
	url := fmt.Sprintf("http://localhost:%d", monitorWebPort)
	fmt.Printf("🐝 Starting Hive Monitor at %s\n", url)
	fmt.Println("Press Ctrl+C to stop")

	go openBrowser(url)

	return monitor.RunWeb(monitorWebPort, monitorHubURL)
}

func openBrowser(url string) {
	var cmd *exec.Cmd
	switch runtime.GOOS {
	case "darwin":
		cmd = exec.Command("open", url)
	case "linux":
		cmd = exec.Command("xdg-open", url)
	case "windows":
		cmd = exec.Command("cmd", "/c", "start", url)
	}
	if cmd != nil {
		_ = cmd.Start()
	}
}
