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

	// Generate .env.generated from hive.yaml config
	if err := cfg.WriteEnvGenerated(hiveDir); err != nil {
		return fmt.Errorf("failed to generate env vars: %w", err)
	}

	// Sync hive.yaml to .hive/ for container access
	if err := syncHiveYAML(); err != nil {
		fmt.Printf("%s\n", ui.Warning("hive.yaml sync: "+err.Error()))
	}

	// Sync host MCPs to .hive/ for container access
	if err := syncHostMCPs(); err != nil {
		fmt.Printf("%s\n", ui.Warning("host MCPs sync: "+err.Error()))
	}

	// Sync CLAUDE.md to .hive/ for container access
	if err := syncProjectCLAUDEmd(); err != nil {
		fmt.Printf("%s\n", ui.Warning("CLAUDE.md sync: "+err.Error()))
	}

	// Step 1: Re-extract embedded files to update scripts/configs
	fmt.Printf("%s ", ui.StyleDim.Render("📦 Updating Hive files..."))

	// Remove old files that might have permission issues
	filesToRemove := []string{
		filepath.Join(hiveDir, "docker"),
		filepath.Join(hiveDir, "entrypoint.sh"),
		filepath.Join(hiveDir, "start-worker.sh"),
		filepath.Join(hiveDir, "worker-daemon.py"),
		filepath.Join(hiveDir, "backends.py"),
		filepath.Join(hiveDir, "tools.py"),
		filepath.Join(hiveDir, "scripts"),
		filepath.Join(hiveDir, "templates"),
	}
	for _, f := range filesToRemove {
		os.RemoveAll(f) // Ignore errors, file might not exist
	}

	if err := extractHiveFiles(""); err != nil {
		return fmt.Errorf("failed to extract hive files: %w", err)
	}
	fmt.Printf("%s\n", ui.StyleGreen.Render("✓"))

	// Step 2: Pull base images (optional)
	if updatePull {
		fmt.Printf("%s ", ui.StyleDim.Render("📥 Pulling latest base images..."))
		pullCmd := exec.Command("docker", "compose", "-f", composeFile, "pull")
		if err := runner.RunQuiet(pullCmd); err != nil {
			fmt.Printf("%s (continuing anyway)\n", ui.StyleYellow.Render("⚠️"))
		} else {
			fmt.Printf("%s\n", ui.StyleGreen.Render("✓"))
		}
	}

	// Step 3: Rebuild images
	fmt.Printf("%s ", ui.StyleDim.Render("🔨 Rebuilding Docker images..."))
	buildArgs := []string{"compose", "-f", composeFile, "build"}
	if updateRebuild {
		buildArgs = append(buildArgs, "--no-cache")
	}
	if updatePull {
		buildArgs = append(buildArgs, "--pull")
	}

	buildCmd := exec.Command("docker", buildArgs...)
	buildCmd.Env = append(os.Environ(),
		fmt.Sprintf("HIVE_DOCKERFILE=%s", dockerfile),
	)

	if err := runner.RunWithTitle(buildCmd, "Docker Build"); err != nil {
		return fmt.Errorf("failed to rebuild images: %w", err)
	}
	fmt.Printf("%s\n", ui.StyleGreen.Render("✓"))

	// Step 4: Stop extra workers that shouldn't be running
	// (e.g., if config says 2 workers but 10 are running)
	for i := cfg.Agents.Workers.Count + 1; i <= 10; i++ {
		containerName := fmt.Sprintf("hive-drone-%d", i)
		stopCmd := exec.Command("docker", "stop", containerName)
		_ = runner.RunQuiet(stopCmd) // Ignore errors if container doesn't exist
		rmCmd := exec.Command("docker", "rm", containerName)
		_ = runner.RunQuiet(rmCmd)
	}

	// Step 5: Recreate containers (preserves volumes)
	// Only start the services defined in config (queen + N workers)
	services := []string{"redis", "queen"}
	for i := 1; i <= cfg.Agents.Workers.Count; i++ {
		services = append(services, fmt.Sprintf("drone-%d", i))
	}

	fmt.Printf("%s ", ui.StyleDim.Render(fmt.Sprintf("🔄 Recreating containers (queen + %d workers)...", cfg.Agents.Workers.Count)))
	upArgs := append([]string{"compose", "-f", composeFile, "up", "-d", "--force-recreate"}, services...)
	upCmd := exec.Command("docker", upArgs...)
	upCmd.Env = append(os.Environ(),
		fmt.Sprintf("HIVE_DOCKERFILE=%s", dockerfile),
	)

	if err := runner.RunWithTitle(upCmd, "Docker Up"); err != nil {
		return fmt.Errorf("failed to recreate containers: %w", err)
	}
	fmt.Printf("%s\n", ui.StyleGreen.Render("✓"))

	// Step 6: Wait for health (optional)
	if updateWait {
		fmt.Printf("%s\n", ui.StyleCyan.Render("⏳ Waiting for containers..."))

		// Build services list based on config
		services := []string{"redis", "queen"}
		for i := 1; i <= cfg.Agents.Workers.Count; i++ {
			services = append(services, fmt.Sprintf("drone-%d", i))
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
