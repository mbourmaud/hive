package cmd

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"text/tabwriter"
	"time"

	"github.com/mbourmaud/hive/internal/agent"
	"github.com/mbourmaud/hive/internal/ui"
	"github.com/spf13/cobra"
)

var agentsCmd = &cobra.Command{
	Use:     "agents",
	Aliases: []string{"ls", "list"},
	Short:   "List running agents",
	Long: `List all running Hive agents.

Examples:
  hive agents           # List all agents
  hive agents --json    # Output as JSON`,
	RunE: runAgents,
}

var agentsJSON bool

func init() {
	rootCmd.AddCommand(agentsCmd)
	agentsCmd.Flags().BoolVar(&agentsJSON, "json", false, "Output as JSON")
}

func runAgents(cmd *cobra.Command, args []string) error {
	agents, err := loadAgentState()
	if err != nil {
		if os.IsNotExist(err) {
			fmt.Println(ui.StyleDim.Render("No agents running"))
			return nil
		}
		return err
	}

	if len(agents) == 0 {
		fmt.Println(ui.StyleDim.Render("No agents running"))
		return nil
	}

	// Check which agents are still alive
	client := agent.NewHTTPClient()
	for i := range agents {
		if client.Health(cmd.Context(), agents[i].Port) {
			agents[i].Status = agent.StatusReady
		} else {
			agents[i].Status = agent.StatusStopped
		}
	}

	if agentsJSON {
		enc := json.NewEncoder(os.Stdout)
		enc.SetIndent("", "  ")
		return enc.Encode(agents)
	}

	// Table output
	w := tabwriter.NewWriter(os.Stdout, 0, 0, 2, ' ', 0)
	fmt.Fprintln(w, "ID\tNAME\tSTATUS\tPORT\tBRANCH\tAGE")

	for _, a := range agents {
		status := ui.StyleGreen.Render("●")
		if a.Status == agent.StatusStopped {
			status = ui.StyleRed.Render("●")
		}

		age := time.Since(a.CreatedAt).Round(time.Second)
		fmt.Fprintf(w, "%s\t%s\t%s\t%d\t%s\t%s\n",
			a.ID,
			a.Name,
			status,
			a.Port,
			a.Branch,
			age,
		)
	}
	w.Flush()

	return nil
}

// State file management

func getStateFilePath() string {
	home, _ := os.UserHomeDir()
	return filepath.Join(home, ".hive", "agents.json")
}

func loadAgentState() ([]*agent.Agent, error) {
	path := getStateFilePath()
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}

	var agents []*agent.Agent
	if err := json.Unmarshal(data, &agents); err != nil {
		return nil, err
	}

	return agents, nil
}

func saveAgentState(agents []*agent.Agent) error {
	path := getStateFilePath()

	// Ensure directory exists
	if err := os.MkdirAll(filepath.Dir(path), 0755); err != nil {
		return err
	}

	data, err := json.MarshalIndent(agents, "", "  ")
	if err != nil {
		return err
	}

	return os.WriteFile(path, data, 0644)
}

func addAgentToState(a *agent.Agent) error {
	agents, err := loadAgentState()
	if err != nil && !os.IsNotExist(err) {
		return err
	}

	// Remove any existing agent with same name
	filtered := make([]*agent.Agent, 0)
	for _, existing := range agents {
		if existing.Name != a.Name {
			filtered = append(filtered, existing)
		}
	}

	filtered = append(filtered, a)
	return saveAgentState(filtered)
}

func removeAgentFromState(name string) error {
	agents, err := loadAgentState()
	if err != nil {
		return err
	}

	filtered := make([]*agent.Agent, 0)
	for _, a := range agents {
		if a.Name != name && a.ID != name {
			filtered = append(filtered, a)
		}
	}

	return saveAgentState(filtered)
}

func getAgentFromState(nameOrID string) (*agent.Agent, error) {
	agents, err := loadAgentState()
	if err != nil {
		return nil, err
	}

	for _, a := range agents {
		if a.Name == nameOrID || a.ID == nameOrID {
			return a, nil
		}
	}

	return nil, fmt.Errorf("agent %s not found", nameOrID)
}
