package cmd

import (
	"fmt"
	"os"
	"strings"

	"github.com/mbourmaud/hive/internal/config"
	"github.com/mbourmaud/hive/internal/hostmcp"
	"github.com/mbourmaud/hive/internal/ui"
	"github.com/spf13/cobra"
)

var mcpCmd = &cobra.Command{
	Use:   "mcp",
	Short: "Manage MCP servers",
	Long:  "Manage Model Context Protocol (MCP) servers running on the host machine",
}

var mcpStatusCmd = &cobra.Command{
	Use:   "status",
	Short: "Show status of host MCP servers",
	RunE: func(cmd *cobra.Command, args []string) error {
		cfg := config.LoadOrDefault()
		manager := hostmcp.NewManager(".hive", cfg)

		fmt.Print(ui.Header("🔌", "Host MCP Status"))

		statuses := manager.Status()
		hasRunning := false

		for _, status := range statuses {
			var icon, stateStr string
			if status.Running {
				icon = ui.StyleGreen.Render("●")
				stateStr = ui.StyleGreen.Render("running")
				hasRunning = true
			} else {
				icon = ui.StyleDim.Render("○")
				stateStr = ui.StyleDim.Render("stopped")
			}

			// Check if enabled in config
			var enabledStr string
			switch status.Type {
			case hostmcp.MCPPlaywright:
				if cfg.HostMCPs.IsPlaywrightEnabled() {
					enabledStr = " (enabled)"
				} else {
					enabledStr = ui.StyleDim.Render(" (disabled)")
				}
			case hostmcp.MCPIOS:
				if cfg.HostMCPs.IsIOSEnabled() {
					enabledStr = " (enabled)"
				} else {
					enabledStr = ui.StyleDim.Render(" (disabled)")
				}
			case hostmcp.MCPClipboard:
				if cfg.HostMCPs.IsClipboardEnabled() {
					enabledStr = " (enabled)"
				} else {
					enabledStr = ui.StyleDim.Render(" (disabled)")
				}
			}

			pidStr := ""
			if status.Running {
				pidStr = fmt.Sprintf(" [PID: %d]", status.PID)
			}

			fmt.Printf("  %s %s: %s on port %d%s%s\n",
				icon, status.Type, stateStr, status.Port, pidStr, enabledStr)
		}

		if !hasRunning {
			fmt.Printf("\n%s\n", ui.StyleDim.Render("No host MCPs are running."))
			fmt.Printf("%s\n", ui.StyleDim.Render("Configure host_mcps in hive.yaml and run 'hive start' to enable."))
		}

		fmt.Println()
		return nil
	},
}

var mcpListCmd = &cobra.Command{
	Use:   "list",
	Short: "List configured MCP servers",
	RunE: func(cmd *cobra.Command, args []string) error {
		cfg := config.LoadOrDefault()

		fmt.Print(ui.Header("📋", "Configured MCPs"))

		// Host MCPs
		fmt.Printf("%s\n", ui.StyleBold.Render("Host MCPs (run on macOS host):"))
		if cfg.HostMCPs.IsPlaywrightEnabled() {
			browser := cfg.HostMCPs.GetPlaywrightBrowser()
			headless := "headless"
			if !cfg.HostMCPs.IsPlaywrightHeadless() {
				headless = "visible"
			}
			fmt.Printf("  • playwright: port %d, %s, %s browser\n",
				cfg.HostMCPs.GetPlaywrightPort(), headless, browser)
		}
		if cfg.HostMCPs.IsIOSEnabled() {
			fmt.Printf("  • ios: port %d (xcrun simctl wrapper)\n",
				cfg.HostMCPs.GetIOSPort())
		}
		if cfg.HostMCPs.IsClipboardEnabled() {
			pngpasteStatus := "pngpaste: not installed"
			if hostmcp.CheckPngpasteInstalled() {
				pngpasteStatus = "pngpaste: installed"
			}
			fmt.Printf("  • clipboard: port %d (%s)\n",
				cfg.HostMCPs.GetClipboardPort(), pngpasteStatus)
		}
		if !cfg.HostMCPs.IsPlaywrightEnabled() && !cfg.HostMCPs.IsIOSEnabled() && !cfg.HostMCPs.IsClipboardEnabled() {
			fmt.Printf("  %s\n", ui.StyleDim.Render("(none configured)"))
		}

		// Container MCPs
		fmt.Printf("\n%s\n", ui.StyleBold.Render("Container MCPs (run inside Docker):"))
		if len(cfg.MCPs) > 0 {
			for name, mcp := range cfg.MCPs {
				if mcp.Package != "" {
					fmt.Printf("  • %s: %s\n", name, mcp.Package)
				} else if mcp.Command != "" {
					fmt.Printf("  • %s: %s %s\n", name, mcp.Command, strings.Join(mcp.Args, " "))
				}
			}
		} else {
			fmt.Printf("  %s\n", ui.StyleDim.Render("(none configured in hive.yaml)"))
		}

		// Built-in
		fmt.Printf("\n%s\n", ui.StyleBold.Render("Built-in MCPs (always available):"))
		fmt.Printf("  • hive: Task orchestration and Redis communication\n")

		fmt.Println()
		return nil
	},
}

var mcpStartCmd = &cobra.Command{
	Use:   "start <name>",
	Short: "Start a host MCP server",
	Args:  cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		name := args[0]
		cfg := config.LoadOrDefault()
		manager := hostmcp.NewManager(".hive", cfg)

		if err := manager.EnsureDirs(); err != nil {
			return err
		}

		switch name {
		case "playwright":
			if err := hostmcp.CheckPlaywrightInstalled(); err != nil {
				return err
			}
			fmt.Printf("%s Starting Playwright MCP on port %d...\n",
				ui.StyleCyan.Render("🌐"), cfg.HostMCPs.GetPlaywrightPort())
			if err := manager.StartPlaywright(); err != nil {
				return err
			}
			fmt.Printf("%s Playwright MCP started\n", ui.StyleGreen.Render("✓"))

		case "ios":
			if err := hostmcp.CheckXcodeInstalled(); err != nil {
				return err
			}
			fmt.Printf("%s Starting iOS MCP on port %d...\n",
				ui.StyleCyan.Render("📱"), cfg.HostMCPs.GetIOSPort())
			if err := manager.StartIOS(); err != nil {
				return err
			}
			fmt.Printf("%s iOS MCP started\n", ui.StyleGreen.Render("✓"))

		case "clipboard":
			fmt.Printf("%s Starting Clipboard MCP on port %d...\n",
				ui.StyleCyan.Render("📋"), cfg.HostMCPs.GetClipboardPort())
			if !hostmcp.CheckPngpasteInstalled() {
				fmt.Printf("%s\n", ui.StyleYellow.Render("⚠ pngpaste not installed. Image clipboard support will be disabled."))
				fmt.Printf("  Install with: brew install pngpaste\n")
			}
			if err := manager.StartClipboard(); err != nil {
				return err
			}
			fmt.Printf("%s Clipboard MCP started\n", ui.StyleGreen.Render("✓"))

		default:
			return fmt.Errorf("unknown host MCP: %s (available: playwright, ios, clipboard)", name)
		}

		return nil
	},
}

var mcpStopCmd = &cobra.Command{
	Use:   "stop <name>",
	Short: "Stop a host MCP server",
	Args:  cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		name := args[0]
		cfg := config.LoadOrDefault()
		manager := hostmcp.NewManager(".hive", cfg)

		var mcpType hostmcp.MCPType
		switch name {
		case "playwright":
			mcpType = hostmcp.MCPPlaywright
		case "ios":
			mcpType = hostmcp.MCPIOS
		case "clipboard":
			mcpType = hostmcp.MCPClipboard
		default:
			return fmt.Errorf("unknown host MCP: %s (available: playwright, ios, clipboard)", name)
		}

		fmt.Printf("%s Stopping %s MCP...\n", ui.StyleCyan.Render("🔌"), name)
		if err := manager.StopMCP(mcpType); err != nil {
			return err
		}
		fmt.Printf("%s %s MCP stopped\n", ui.StyleGreen.Render("✓"), name)

		return nil
	},
}

var mcpLogsCmd = &cobra.Command{
	Use:   "logs <name>",
	Short: "View logs of a host MCP server",
	Args:  cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		name := args[0]
		lines, _ := cmd.Flags().GetInt("lines")

		cfg := config.LoadOrDefault()
		manager := hostmcp.NewManager(".hive", cfg)

		var mcpType hostmcp.MCPType
		switch name {
		case "playwright":
			mcpType = hostmcp.MCPPlaywright
		case "ios":
			mcpType = hostmcp.MCPIOS
		case "clipboard":
			mcpType = hostmcp.MCPClipboard
		default:
			return fmt.Errorf("unknown host MCP: %s (available: playwright, ios, clipboard)", name)
		}

		logs, err := manager.GetLogs(mcpType, lines)
		if err != nil {
			if os.IsNotExist(err) {
				fmt.Printf("%s\n", ui.StyleDim.Render("No logs available. MCP may not have been started yet."))
				return nil
			}
			return err
		}

		fmt.Printf("%s %s MCP Logs (last %d lines):\n\n", ui.StyleBold.Render("📜"), name, lines)
		fmt.Println(logs)

		return nil
	},
}

func init() {
	rootCmd.AddCommand(mcpCmd)
	mcpCmd.AddCommand(mcpStatusCmd)
	mcpCmd.AddCommand(mcpListCmd)
	mcpCmd.AddCommand(mcpStartCmd)
	mcpCmd.AddCommand(mcpStopCmd)
	mcpCmd.AddCommand(mcpLogsCmd)

	mcpLogsCmd.Flags().IntP("lines", "n", 50, "Number of log lines to show")
}
