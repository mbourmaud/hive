package cmd

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"time"

	"github.com/mbourmaud/hive/internal/config"
	"github.com/mbourmaud/hive/internal/shell"
	"github.com/mbourmaud/hive/internal/ui"
	"github.com/spf13/cobra"
)

var (
	updateRebuild bool
	updatePull    bool
	updateWait    bool
)

var updateCmd = &cobra.Command{
	Use:   "update",
	Short: "Update Hive containers with latest images",
	Long: `Update Docker images and restart containers without losing data.

This command rebuilds Docker images from the current Dockerfile and recreates
containers while preserving all volumes (workspaces, history, caches).

Use this after pulling Hive updates instead of 'hive clean && hive init'.

Examples:
  hive update                    # Smart rebuild (uses cache)
  hive update --rebuild          # Force rebuild from scratch
  hive update --pull             # Pull latest base images first
  hive update --rebuild --pull   # Complete refresh with latest bases`,
	RunE: runUpdate,
}

func init() {
	rootCmd.AddCommand(updateCmd)
	updateCmd.Flags().BoolVar(&updateRebuild, "rebuild", false, "Rebuild from scratch (--no-cache)")
	updateCmd.Flags().BoolVar(&updatePull, "pull", false, "Pull latest base images before building")
	updateCmd.Flags().BoolVar(&updateWait, "wait", false, "Wait for containers to be healthy")
}

func runUpdate(cmd *cobra.Command, args []string) error {
	fmt.Print(ui.Header("📦", "Updating Hive"))

	// Check if .hive directory exists
	hiveDir := ".hive"
	if _, err := os.Stat(hiveDir); os.IsNotExist(err) {
		return fmt.Errorf("Hive not initialized. Run 'hive init' first")
	}

	// Load configuration
	cfg := config.LoadOrDefault()

	// Get dockerfile from config
	dockerfile := cfg.Agents.Queen.Dockerfile
	if dockerfile == "" {
		dockerfile = "docker/Dockerfile.node"
	}

	runner := shell.NewRunner(DebugMode)
	composeFile := filepath.Join(hiveDir, "docker-compose.yml")

	// Step 1: Pull base images (optional)
	if updatePull {
		fmt.Printf("%s ", ui.StyleDim.Render("📥 Pulling latest base images..."))
		pullCmd := exec.Command("docker", "compose", "-f", composeFile, "pull")
		pullCmd.Dir = hiveDir
		if err := runner.RunQuiet(pullCmd); err != nil {
			fmt.Printf("%s (continuing anyway)\n", ui.StyleYellow.Render("⚠️"))
		} else {
			fmt.Printf("%s\n", ui.StyleGreen.Render("✓"))
		}
	}

	// Step 2: Rebuild images
	fmt.Printf("%s ", ui.StyleDim.Render("🔨 Rebuilding Docker images..."))
	buildArgs := []string{"compose", "-f", composeFile, "build"}
	if updateRebuild {
		buildArgs = append(buildArgs, "--no-cache")
	}
	if updatePull {
		buildArgs = append(buildArgs, "--pull")
	}

	buildCmd := exec.Command("docker", buildArgs...)
	buildCmd.Dir = hiveDir
	buildCmd.Env = append(os.Environ(),
		fmt.Sprintf("HIVE_DOCKERFILE=%s", dockerfile),
	)

	if err := runner.RunWithTitle(buildCmd, "Docker Build"); err != nil {
		return fmt.Errorf("failed to rebuild images: %w", err)
	}
	fmt.Printf("%s\n", ui.StyleGreen.Render("✓"))

	// Step 3: Recreate containers (preserves volumes)
	fmt.Printf("%s ", ui.StyleDim.Render("🔄 Recreating containers..."))
	upCmd := exec.Command("docker", "compose", "-f", composeFile, "up", "-d", "--force-recreate")
	upCmd.Dir = hiveDir
	upCmd.Env = append(os.Environ(),
		fmt.Sprintf("HIVE_DOCKERFILE=%s", dockerfile),
	)

	if err := runner.RunWithTitle(upCmd, "Docker Up"); err != nil {
		return fmt.Errorf("failed to recreate containers: %w", err)
	}
	fmt.Printf("%s\n", ui.StyleGreen.Render("✓"))

	// Step 4: Wait for health (optional)
	if updateWait {
		fmt.Printf("%s\n", ui.StyleCyan.Render("⏳ Waiting for containers..."))

		// Build services list based on config
		services := []string{"redis", "queen"}
		for i := 1; i <= cfg.Agents.Workers.Count; i++ {
			services = append(services, fmt.Sprintf("agent-%d", i))
		}

		if err := waitForContainersReady(runner, services, 60*time.Second); err != nil {
			return err
		}
		fmt.Println()
	}

	fmt.Printf("\n%s\n", ui.Success("✅ Hive updated successfully!"))
	fmt.Printf("\n%s\n", ui.StyleDim.Render("All data preserved (workspaces, history, caches)"))
	fmt.Printf("%s\n\n", ui.StyleDim.Render("Run 'hive status' to verify containers are running"))

	// Next steps
	steps := []ui.Step{
		{Command: "hive status", Description: "Check status"},
		{Command: "hive connect queen", Description: "Connect to queen"},
	}
	fmt.Print(ui.NextSteps(steps))

	return nil
}
