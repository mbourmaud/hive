package cmd

import (
	"fmt"
	"os"
	"text/tabwriter"
	"time"

	"github.com/mbourmaud/hive/internal/agent"
	"github.com/mbourmaud/hive/internal/ui"
	"github.com/spf13/cobra"
)

var statusCmd = &cobra.Command{
	Use:   "status",
	Short: "Show Hive status",
	Long: `Display the current status of Hive agents.

Examples:
  hive status`,
	RunE: runStatus,
}

func init() {
	rootCmd.AddCommand(statusCmd)
}

func runStatus(cmd *cobra.Command, args []string) error {
	agents, err := loadAgentState()
	if err != nil {
		if os.IsNotExist(err) {
			fmt.Printf("%s Hive v2\n\n", ui.StyleCyan.Render("🐝"))
			fmt.Println(ui.StyleDim.Render("No agents running"))
			fmt.Println()
			fmt.Printf("Spawn an agent: %s\n", ui.StyleCyan.Render("hive spawn <name>"))
			return nil
		}
		return err
	}

	fmt.Printf("%s Hive v2\n\n", ui.StyleCyan.Render("🐝"))

	if len(agents) == 0 {
		fmt.Println(ui.StyleDim.Render("No agents running"))
		fmt.Println()
		fmt.Printf("Spawn an agent: %s\n", ui.StyleCyan.Render("hive spawn <name>"))
		return nil
	}

	// Check which agents are still alive
	client := agent.NewHTTPClient()
	running := 0
	for i := range agents {
		if client.Health(cmd.Context(), agents[i].Port) {
			agents[i].Status = agent.StatusReady
			running++
		} else {
			agents[i].Status = agent.StatusStopped
		}
	}

	fmt.Printf("Agents: %s running, %s total\n\n",
		ui.StyleGreen.Render(fmt.Sprintf("%d", running)),
		fmt.Sprintf("%d", len(agents)))

	// Table output
	w := tabwriter.NewWriter(os.Stdout, 0, 0, 2, ' ', 0)
	fmt.Fprintln(w, "NAME\tSTATUS\tPORT\tBRANCH\tAGE")

	for _, a := range agents {
		status := ui.StyleGreen.Render("running")
		if a.Status == agent.StatusStopped {
			status = ui.StyleRed.Render("stopped")
		}

		age := formatDuration(time.Since(a.CreatedAt))
		fmt.Fprintf(w, "%s\t%s\t%d\t%s\t%s\n",
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

func formatDuration(d time.Duration) string {
	if d < time.Minute {
		return fmt.Sprintf("%ds", int(d.Seconds()))
	}
	if d < time.Hour {
		return fmt.Sprintf("%dm", int(d.Minutes()))
	}
	if d < 24*time.Hour {
		return fmt.Sprintf("%dh", int(d.Hours()))
	}
	return fmt.Sprintf("%dd", int(d.Hours()/24))
}
