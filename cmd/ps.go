package cmd

import (
	"bufio"
	"fmt"
	"os/exec"
	"regexp"
	"strconv"
	"strings"

	"github.com/mbourmaud/hive/internal/ui"
	"github.com/spf13/cobra"
)

var psCmd = &cobra.Command{
	Use:   "ps [agent]",
	Short: "Show processes and listening ports in agent containers",
	Long: `Show running processes and listening ports inside agent containers.

Without arguments, shows info for all running agents.
With an agent name, shows detailed info for that agent.

Examples:
  hive ps           # Show summary for all agents
  hive ps drone-1   # Show details for drone-1
  hive ps queen     # Show details for queen`,
	Args: cobra.MaximumNArgs(1),
	RunE: runPs,
}

var psVerbose bool

func init() {
	rootCmd.AddCommand(psCmd)
	psCmd.Flags().BoolVarP(&psVerbose, "verbose", "v", false, "Show all processes, not just user processes")
}

func runPs(cmd *cobra.Command, args []string) error {
	// Get running hive containers
	listCmd := exec.Command("docker", "ps", "--filter", "name=hive-", "--format", "{{.Names}}")
	output, err := listCmd.Output()
	if err != nil {
		return fmt.Errorf("failed to list containers: %w", err)
	}

	containers := strings.Split(strings.TrimSpace(string(output)), "\n")
	if len(containers) == 0 || containers[0] == "" {
		fmt.Println("No hive containers running")
		return nil
	}

	// Filter by agent if specified
	if len(args) > 0 {
		agent := args[0]
		containerName := "hive-" + agent
		found := false
		for _, c := range containers {
			if c == containerName {
				found = true
				containers = []string{containerName}
				break
			}
		}
		if !found {
			return fmt.Errorf("container %s not found or not running", containerName)
		}
	}

	fmt.Print(ui.Header("📊", "Agent Processes"))

	for _, container := range containers {
		if container == "" || container == "hive-redis" {
			continue
		}

		agentName := strings.TrimPrefix(container, "hive-")
		fmt.Printf("\n%s\n", ui.StyleBold.Render("▸ "+agentName))

		// Get listening ports
		ports := getListeningPorts(container)
		if len(ports) > 0 {
			fmt.Printf("  %s\n", ui.StyleCyan.Render("Listening ports:"))
			for _, p := range ports {
				fmt.Printf("    %s %s\n", ui.StyleGreen.Render(fmt.Sprintf(":%d", p.Port)), ui.StyleDim.Render(p.Process))
			}
		} else {
			fmt.Printf("  %s\n", ui.StyleDim.Render("No ports listening"))
		}

		// Get user processes
		processes := getUserProcesses(container, psVerbose)
		if len(processes) > 0 {
			fmt.Printf("  %s\n", ui.StyleCyan.Render("Processes:"))
			for _, p := range processes {
				fmt.Printf("    %s %s\n", ui.StyleYellow.Render(fmt.Sprintf("PID %s", p.PID)), p.Command)
			}
		}
	}

	// Show expose hint
	fmt.Printf("\n%s\n", ui.StyleDim.Render("To expose ports: hive expose <agent> --ports <port1,port2>"))
	fmt.Println()

	return nil
}

type PortInfo struct {
	Port    int
	Process string
}

type ProcessInfo struct {
	PID     string
	Command string
}

func getListeningPorts(container string) []PortInfo {
	var ports []PortInfo

	// Use /proc/net/tcp and /proc/net/tcp6 to find listening ports
	// Then correlate with /proc/*/fd to find the process
	script := `
	for proto in tcp tcp6; do
		if [ -f /proc/net/$proto ]; then
			awk 'NR>1 && $4=="0A" {print $2}' /proc/net/$proto | cut -d: -f2 | while read hex; do
				port=$((16#$hex))
				if [ $port -gt 0 ] && [ $port -lt 65536 ]; then
					# Find process using this port
					proc=""
					for pid in $(ls /proc 2>/dev/null | grep -E '^[0-9]+$'); do
						if ls -la /proc/$pid/fd 2>/dev/null | grep -q "socket:"; then
							cmdline=$(cat /proc/$pid/cmdline 2>/dev/null | tr '\0' ' ' | cut -c1-50)
							if [ -n "$cmdline" ]; then
								proc="$cmdline"
								break
							fi
						fi
					done
					echo "$port|$proc"
				fi
			done
		fi
	done | sort -t'|' -k1 -n | uniq
	`

	cmd := exec.Command("docker", "exec", container, "sh", "-c", script)
	output, err := cmd.Output()
	if err != nil {
		return ports
	}

	seen := make(map[int]bool)
	scanner := bufio.NewScanner(strings.NewReader(string(output)))
	for scanner.Scan() {
		line := scanner.Text()
		parts := strings.SplitN(line, "|", 2)
		if len(parts) >= 1 {
			port, err := strconv.Atoi(parts[0])
			if err != nil || port <= 0 || port > 65535 || seen[port] {
				continue
			}
			// Filter out common non-user ports
			if port < 1024 && port != 80 && port != 443 {
				continue
			}
			seen[port] = true
			process := ""
			if len(parts) > 1 {
				process = strings.TrimSpace(parts[1])
			}
			ports = append(ports, PortInfo{Port: port, Process: process})
		}
	}

	return ports
}

func getUserProcesses(container string, verbose bool) []ProcessInfo {
	var processes []ProcessInfo

	// Get processes, filter out system processes
	cmd := exec.Command("docker", "exec", container, "ps", "aux")
	output, err := cmd.Output()
	if err != nil {
		return processes
	}

	// Patterns to exclude (system/shell processes)
	excludePatterns := []*regexp.Regexp{
		regexp.MustCompile(`^/bin/(ba)?sh`),
		regexp.MustCompile(`^sleep`),
		regexp.MustCompile(`^ps aux`),
		regexp.MustCompile(`^/home/agent/start-`),
		regexp.MustCompile(`^python.*worker-daemon`),
		regexp.MustCompile(`^sh -c`),
		regexp.MustCompile(`^tee`),
	}

	// Patterns to always include (dev servers)
	includePatterns := []*regexp.Regexp{
		regexp.MustCompile(`node`),
		regexp.MustCompile(`npm`),
		regexp.MustCompile(`pnpm`),
		regexp.MustCompile(`expo`),
		regexp.MustCompile(`next`),
		regexp.MustCompile(`vite`),
		regexp.MustCompile(`webpack`),
		regexp.MustCompile(`python.*server`),
		regexp.MustCompile(`uvicorn`),
		regexp.MustCompile(`gunicorn`),
		regexp.MustCompile(`flask`),
		regexp.MustCompile(`django`),
	}

	scanner := bufio.NewScanner(strings.NewReader(string(output)))
	lineNum := 0
	for scanner.Scan() {
		lineNum++
		if lineNum == 1 {
			continue // Skip header
		}

		line := scanner.Text()
		fields := strings.Fields(line)
		if len(fields) < 11 {
			continue
		}

		pid := fields[1]
		command := strings.Join(fields[10:], " ")

		// Check if should include
		shouldInclude := verbose
		if !shouldInclude {
			for _, p := range includePatterns {
				if p.MatchString(command) {
					shouldInclude = true
					break
				}
			}
		}

		// Check if should exclude
		if !verbose {
			for _, p := range excludePatterns {
				if p.MatchString(command) {
					shouldInclude = false
					break
				}
			}
		}

		if shouldInclude {
			// Truncate long commands
			if len(command) > 60 {
				command = command[:57] + "..."
			}
			processes = append(processes, ProcessInfo{PID: pid, Command: command})
		}
	}

	return processes
}
