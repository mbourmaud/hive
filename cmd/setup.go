package cmd

import (
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"

	"github.com/mbourmaud/hive/internal/ui"
	"github.com/spf13/cobra"
)

var setupCmd = &cobra.Command{
	Use:   "setup",
	Short: "Install Hive dependencies (agentapi, claude)",
	Long: `Automatically install all dependencies required by Hive.

This command detects your OS and architecture, then installs:
  - agentapi: HTTP control layer for Claude Code
  - claude: Claude Code CLI (via npm if not found)

Examples:
  hive setup              # Install all dependencies
  hive setup --check      # Only check what's installed`,
	RunE: runSetup,
}

var installCmd = &cobra.Command{
	Use:   "install [component]",
	Short: "Install Hive components",
	Long: `Install optional Hive components.

Available components:
  desktop    Install the Hive Monitor desktop app (macOS)

Examples:
  hive install desktop    # Install the desktop app`,
}

var installDesktopCmd = &cobra.Command{
	Use:   "desktop",
	Short: "Install Hive Monitor desktop app",
	Long: `Download and install the Hive Monitor desktop application.

On macOS, this installs Hive Monitor.app to /Applications.
The app provides a graphical interface to monitor and manage your Hive agents.`,
	RunE: runInstallDesktop,
}

var (
	setupCheckOnly bool
)

func init() {
	rootCmd.AddCommand(setupCmd)
	setupCmd.Flags().BoolVar(&setupCheckOnly, "check", false, "Only check dependencies, don't install")

	rootCmd.AddCommand(installCmd)
	installCmd.AddCommand(installDesktopCmd)
}

type dependency struct {
	name        string
	check       func() (string, bool)
	install     func() error
	description string
}

func runSetup(cmd *cobra.Command, args []string) error {
	fmt.Printf("%s Hive Setup\n\n", ui.StyleCyan.Render("🐝"))

	osName := runtime.GOOS
	arch := runtime.GOARCH

	fmt.Printf("  %s %s/%s\n\n", ui.StyleDim.Render("System:"), osName, arch)

	deps := []dependency{
		{
			name:        "agentapi",
			description: "HTTP control layer for Claude Code",
			check:       checkAgentAPI,
			install:     installAgentAPI,
		},
		{
			name:        "claude",
			description: "Claude Code CLI",
			check:       checkClaude,
			install:     installClaude,
		},
	}

	allInstalled := true
	needsInstall := []dependency{}

	for _, dep := range deps {
		version, installed := dep.check()
		if installed {
			fmt.Printf("  %s %s %s\n", ui.StyleGreen.Render("✓"), dep.name, ui.StyleDim.Render(version))
		} else {
			fmt.Printf("  %s %s %s\n", ui.StyleYellow.Render("✗"), dep.name, ui.StyleDim.Render("not found"))
			allInstalled = false
			needsInstall = append(needsInstall, dep)
		}
	}

	fmt.Println()

	if allInstalled {
		fmt.Printf("%s All dependencies installed!\n", ui.StyleGreen.Render("✓"))
		return nil
	}

	if setupCheckOnly {
		fmt.Printf("%s Some dependencies are missing. Run 'hive setup' to install.\n", ui.StyleYellow.Render("⚠️"))
		return nil
	}

	fmt.Printf("Installing missing dependencies...\n\n")

	for _, dep := range needsInstall {
		fmt.Printf("  %s Installing %s...\n", ui.StyleCyan.Render("→"), dep.name)
		if err := dep.install(); err != nil {
			fmt.Printf("  %s Failed to install %s: %v\n", ui.StyleRed.Render("✗"), dep.name, err)
			fmt.Printf("    %s\n", ui.StyleDim.Render(dep.description))
			continue
		}
		version, ok := dep.check()
		if ok {
			fmt.Printf("  %s %s installed %s\n", ui.StyleGreen.Render("✓"), dep.name, ui.StyleDim.Render(version))
		}
	}

	fmt.Println()
	fmt.Printf("%s Setup complete! Run 'hive init' in your project.\n", ui.StyleGreen.Render("✓"))

	return nil
}

func checkAgentAPI() (string, bool) {
	path, err := exec.LookPath("agentapi")
	if err != nil {
		return "", false
	}
	out, err := exec.Command(path, "--version").CombinedOutput()
	if err != nil {
		return path, true
	}
	version := strings.TrimSpace(string(out))
	if version == "" {
		return "installed", true
	}
	return version, true
}

func checkClaude() (string, bool) {
	path, err := exec.LookPath("claude")
	if err != nil {
		return "", false
	}
	out, err := exec.Command(path, "--version").CombinedOutput()
	if err != nil {
		return path, true
	}
	version := strings.TrimSpace(string(out))
	lines := strings.Split(version, "\n")
	if len(lines) > 0 {
		return lines[0], true
	}
	return "installed", true
}

func installAgentAPI() error {
	osName := runtime.GOOS
	arch := runtime.GOARCH

	if arch == "amd64" {
		arch = "amd64"
	} else if arch == "arm64" {
		arch = "arm64"
	} else {
		return fmt.Errorf("unsupported architecture: %s", arch)
	}

	if osName != "darwin" && osName != "linux" {
		return fmt.Errorf("unsupported OS: %s (only darwin and linux supported)", osName)
	}

	url := fmt.Sprintf("https://github.com/coder/agentapi/releases/latest/download/agentapi-%s-%s", osName, arch)

	installDir := getInstallDir()
	if err := os.MkdirAll(installDir, 0755); err != nil {
		return fmt.Errorf("failed to create install directory: %w", err)
	}

	destPath := filepath.Join(installDir, "agentapi")

	resp, err := http.Get(url)
	if err != nil {
		return fmt.Errorf("failed to download: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("download failed with status: %s", resp.Status)
	}

	out, err := os.Create(destPath)
	if err != nil {
		return fmt.Errorf("failed to create file: %w", err)
	}
	defer out.Close()

	if _, err := io.Copy(out, resp.Body); err != nil {
		return fmt.Errorf("failed to write file: %w", err)
	}

	if err := os.Chmod(destPath, 0755); err != nil {
		return fmt.Errorf("failed to make executable: %w", err)
	}

	addToPathHint(installDir)

	return nil
}

func installClaude() error {
	if _, err := exec.LookPath("npm"); err != nil {
		return fmt.Errorf("npm not found - install Node.js first: https://nodejs.org")
	}

	cmd := exec.Command("npm", "install", "-g", "@anthropic-ai/claude-code")
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	if err := cmd.Run(); err != nil {
		return fmt.Errorf("npm install failed: %w", err)
	}

	return nil
}

func getInstallDir() string {
	if dir := os.Getenv("HIVE_BIN_DIR"); dir != "" {
		return dir
	}

	home, _ := os.UserHomeDir()

	candidates := []string{
		filepath.Join(home, ".local", "bin"),
		filepath.Join(home, "bin"),
		filepath.Join(home, "go", "bin"),
	}

	for _, dir := range candidates {
		if _, err := os.Stat(dir); err == nil {
			if isInPath(dir) {
				return dir
			}
		}
	}

	return filepath.Join(home, ".local", "bin")
}

func isInPath(dir string) bool {
	pathEnv := os.Getenv("PATH")
	paths := strings.Split(pathEnv, string(os.PathListSeparator))
	for _, p := range paths {
		if p == dir {
			return true
		}
	}
	return false
}

func addToPathHint(dir string) {
	if isInPath(dir) {
		return
	}

	shell := os.Getenv("SHELL")
	var rcFile string

	switch {
	case strings.Contains(shell, "zsh"):
		rcFile = "~/.zshrc"
	case strings.Contains(shell, "bash"):
		rcFile = "~/.bashrc"
	default:
		rcFile = "your shell config"
	}

	fmt.Printf("\n    %s Add to PATH: %s\n", ui.StyleYellow.Render("!"), ui.StyleDim.Render(fmt.Sprintf("echo 'export PATH=\"%s:$PATH\"' >> %s", dir, rcFile)))
}

func runInstallDesktop(cmd *cobra.Command, args []string) error {
	fmt.Printf("%s Installing Hive Monitor Desktop\n\n", ui.StyleCyan.Render("🐝"))

	osName := runtime.GOOS
	arch := runtime.GOARCH

	if osName != "darwin" {
		return fmt.Errorf("desktop app is currently only available for macOS")
	}

	fmt.Printf("  %s %s/%s\n\n", ui.StyleDim.Render("System:"), osName, arch)

	var dmgName string
	switch arch {
	case "arm64":
		dmgName = "Hive.Monitor-arm64.dmg"
	case "amd64":
		dmgName = "Hive.Monitor-x64.dmg"
	default:
		return fmt.Errorf("unsupported architecture: %s", arch)
	}

	releaseURL := fmt.Sprintf("https://github.com/mbourmaud/hive/releases/latest/download/%s", dmgName)

	tmpDir, err := os.MkdirTemp("", "hive-desktop-*")
	if err != nil {
		return fmt.Errorf("failed to create temp directory: %w", err)
	}
	defer os.RemoveAll(tmpDir)

	dmgPath := filepath.Join(tmpDir, dmgName)

	fmt.Printf("  %s Downloading %s...\n", ui.StyleCyan.Render("→"), dmgName)

	resp, err := http.Get(releaseURL)
	if err != nil {
		return fmt.Errorf("failed to download: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode == http.StatusNotFound {
		fmt.Printf("  %s Release not found on GitHub\n", ui.StyleYellow.Render("!"))
		fmt.Printf("  %s Building from source...\n\n", ui.StyleCyan.Render("→"))
		return buildDesktopFromSource()
	}

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("download failed with status: %s", resp.Status)
	}

	out, err := os.Create(dmgPath)
	if err != nil {
		return fmt.Errorf("failed to create file: %w", err)
	}

	if _, err := io.Copy(out, resp.Body); err != nil {
		out.Close()
		return fmt.Errorf("failed to write file: %w", err)
	}
	out.Close()

	fmt.Printf("  %s Mounting DMG...\n", ui.StyleCyan.Render("→"))

	mountCmd := exec.Command("hdiutil", "attach", dmgPath, "-nobrowse", "-quiet")
	if err := mountCmd.Run(); err != nil {
		return fmt.Errorf("failed to mount DMG: %w", err)
	}

	mountPoint := "/Volumes/Hive Monitor"
	defer exec.Command("hdiutil", "detach", mountPoint, "-quiet").Run()

	appSrc := filepath.Join(mountPoint, "Hive Monitor.app")
	appDst := "/Applications/Hive Monitor.app"

	if _, err := os.Stat(appDst); err == nil {
		fmt.Printf("  %s Removing existing installation...\n", ui.StyleCyan.Render("→"))
		if err := os.RemoveAll(appDst); err != nil {
			return fmt.Errorf("failed to remove existing app: %w", err)
		}
	}

	fmt.Printf("  %s Installing to /Applications...\n", ui.StyleCyan.Render("→"))

	cpCmd := exec.Command("cp", "-R", appSrc, appDst)
	if err := cpCmd.Run(); err != nil {
		return fmt.Errorf("failed to copy app: %w", err)
	}

	fmt.Println()
	fmt.Printf("%s Hive Monitor installed!\n\n", ui.StyleGreen.Render("✓"))
	fmt.Printf("  Launch from Applications or run:\n")
	fmt.Printf("  %s\n", ui.StyleCyan.Render("open '/Applications/Hive Monitor.app'"))

	return nil
}

func buildDesktopFromSource() error {
	home, _ := os.UserHomeDir()
	hiveDir := findHiveSourceDir(home)

	if hiveDir == "" {
		return fmt.Errorf("hive source not found - clone it first:\n  git clone https://github.com/mbourmaud/hive ~/Projects/hive")
	}

	webDir := filepath.Join(hiveDir, "web")

	if _, err := os.Stat(webDir); os.IsNotExist(err) {
		return fmt.Errorf("web directory not found in %s", hiveDir)
	}

	if _, err := exec.LookPath("npm"); err != nil {
		return fmt.Errorf("npm not found - install Node.js first")
	}

	fmt.Printf("  %s Found source at %s\n", ui.StyleDim.Render("→"), hiveDir)

	fmt.Printf("  %s Installing dependencies...\n", ui.StyleCyan.Render("→"))
	npmInstall := exec.Command("npm", "install")
	npmInstall.Dir = webDir
	npmInstall.Stdout = os.Stdout
	npmInstall.Stderr = os.Stderr
	if err := npmInstall.Run(); err != nil {
		return fmt.Errorf("npm install failed: %w", err)
	}

	fmt.Printf("  %s Building web UI...\n", ui.StyleCyan.Render("→"))
	npmBuild := exec.Command("npm", "run", "build")
	npmBuild.Dir = webDir
	npmBuild.Stdout = os.Stdout
	npmBuild.Stderr = os.Stderr
	if err := npmBuild.Run(); err != nil {
		return fmt.Errorf("build failed: %w", err)
	}

	fmt.Printf("  %s Building Electron app...\n", ui.StyleCyan.Render("→"))

	arch := runtime.GOARCH
	electronBuild := exec.Command("npx", "electron-builder", "--mac", "--"+arch)
	electronBuild.Dir = webDir
	electronBuild.Stdout = os.Stdout
	electronBuild.Stderr = os.Stderr
	if err := electronBuild.Run(); err != nil {
		return fmt.Errorf("electron-builder failed: %w", err)
	}

	appSrc := filepath.Join(webDir, "release", "mac-"+arch, "Hive Monitor.app")
	appDst := "/Applications/Hive Monitor.app"

	if _, err := os.Stat(appDst); err == nil {
		os.RemoveAll(appDst)
	}

	fmt.Printf("  %s Installing to /Applications...\n", ui.StyleCyan.Render("→"))

	cpCmd := exec.Command("cp", "-R", appSrc, appDst)
	if err := cpCmd.Run(); err != nil {
		return fmt.Errorf("failed to copy app: %w", err)
	}

	fmt.Println()
	fmt.Printf("%s Hive Monitor installed!\n\n", ui.StyleGreen.Render("✓"))
	fmt.Printf("  Launch from Applications or run:\n")
	fmt.Printf("  %s\n", ui.StyleCyan.Render("open '/Applications/Hive Monitor.app'"))

	return nil
}

func findHiveSourceDir(home string) string {
	candidates := []string{
		filepath.Join(home, "Projects", "hive"),
		filepath.Join(home, "projects", "hive"),
		filepath.Join(home, "dev", "hive"),
		filepath.Join(home, "src", "hive"),
		filepath.Join(home, "hive"),
	}

	for _, dir := range candidates {
		if _, err := os.Stat(filepath.Join(dir, "go.mod")); err == nil {
			return dir
		}
	}

	return ""
}
