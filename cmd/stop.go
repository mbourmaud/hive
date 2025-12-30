package cmd

import (
	"fmt"
	"os/exec"

	"github.com/mbourmaud/hive/internal/config"
	"github.com/mbourmaud/hive/internal/hostmcp"
	"github.com/mbourmaud/hive/internal/shell"
	"github.com/mbourmaud/hive/internal/ui"
	"github.com/spf13/cobra"
)

var stopCmd = &cobra.Command{
	Use:   "stop",
	Short: "Stop all hive containers",
	Long:  "Stop all running hive containers and host MCPs",
	RunE: func(cmd *cobra.Command, args []string) error {
		// Header
		fmt.Print(ui.Header("🛑", "Stopping Hive"))

		// Create shell runner with debug mode
		runner := shell.NewRunner(DebugMode)

		// Stop containers
		dockerCmd := exec.Command("docker", "compose", "-f", ".hive/docker-compose.yml", "down")
		if err := runner.RunWithTitle(dockerCmd, "Docker Compose Stop"); err != nil {
			return fmt.Errorf("failed to stop containers: %w", err)
		}

		// Stop host MCPs
		cfg := config.LoadOrDefault()
		if cfg.HostMCPs.IsPlaywrightEnabled() || cfg.HostMCPs.IsIOSEnabled() || cfg.HostMCPs.IsClipboardEnabled() {
			fmt.Printf("\n%s Stopping host MCPs...\n", ui.StyleCyan.Render("🔌"))
			mcpManager := hostmcp.NewManager(".hive", cfg)
			if err := mcpManager.StopAll(); err != nil {
				fmt.Printf("%s\n", ui.Warning("Host MCPs: "+err.Error()))
			} else {
				fmt.Printf("%s Host MCPs stopped\n", ui.StyleGreen.Render("✓"))
			}
		}

		// Success message
		fmt.Printf("\n%s\n\n", ui.Success("Hive stopped successfully!"))
		return nil
	},
}

func init() {
	rootCmd.AddCommand(stopCmd)
}
