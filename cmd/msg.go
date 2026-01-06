package cmd

import (
	"fmt"
	"strings"

	"github.com/mbourmaud/hive/internal/agent"
	"github.com/mbourmaud/hive/internal/ui"
	"github.com/spf13/cobra"
)

var msgCmd = &cobra.Command{
	Use:   "msg <agent> <message>",
	Short: "Send a message to an agent",
	Long: `Send a message to a running agent.

Examples:
  hive msg front "Fix the login bug"
  hive msg back "Add user authentication endpoint"`,
	Args: cobra.MinimumNArgs(2),
	RunE: runMsg,
}

func init() {
	rootCmd.AddCommand(msgCmd)
}

func runMsg(cmd *cobra.Command, args []string) error {
	agentName := args[0]
	message := strings.Join(args[1:], " ")

	// Get agent from state
	a, err := getAgentFromState(agentName)
	if err != nil {
		return fmt.Errorf("agent not found: %w", err)
	}

	// Check if agent is alive
	client := agent.NewHTTPClient()
	if !client.Health(cmd.Context(), a.Port) {
		return fmt.Errorf("agent %s is not responding (port %d)", agentName, a.Port)
	}

	fmt.Printf("%s Sending message to %s...\n", ui.StyleCyan.Render("📨"), ui.StyleBold.Render(a.Name))

	// Send message
	if err := client.SendMessage(cmd.Context(), a.Port, message); err != nil {
		return fmt.Errorf("failed to send message: %w", err)
	}

	fmt.Printf("%s Message sent!\n", ui.StyleGreen.Render("✓"))
	return nil
}

var convCmd = &cobra.Command{
	Use:     "conv <agent>",
	Aliases: []string{"conversation", "history"},
	Short:   "Show conversation history with an agent",
	Long: `Display the conversation history with an agent.

Examples:
  hive conv front
  hive conv back --json`,
	Args: cobra.ExactArgs(1),
	RunE: runConv,
}

var convJSON bool

func init() {
	rootCmd.AddCommand(convCmd)
	convCmd.Flags().BoolVar(&convJSON, "json", false, "Output as JSON")
}

func runConv(cmd *cobra.Command, args []string) error {
	agentName := args[0]

	// Get agent from state
	a, err := getAgentFromState(agentName)
	if err != nil {
		return fmt.Errorf("agent not found: %w", err)
	}

	// Check if agent is alive
	client := agent.NewHTTPClient()
	if !client.Health(cmd.Context(), a.Port) {
		return fmt.Errorf("agent %s is not responding (port %d)", agentName, a.Port)
	}

	// Get messages
	messages, err := client.GetMessages(cmd.Context(), a.Port)
	if err != nil {
		return fmt.Errorf("failed to get conversation: %w", err)
	}

	if len(messages) == 0 {
		fmt.Println(ui.StyleDim.Render("No messages yet"))
		return nil
	}

	fmt.Printf("%s Conversation with %s:\n\n", ui.StyleCyan.Render("💬"), ui.StyleBold.Render(a.Name))

	for _, msg := range messages {
		var prefix string
		if msg.Role == "user" {
			prefix = ui.StyleGreen.Render("You: ")
		} else {
			prefix = ui.StyleCyan.Render(a.Name + ": ")
		}

		// Truncate long messages
		content := msg.Content
		if len(content) > 500 {
			content = content[:500] + "..."
		}

		fmt.Printf("%s%s\n\n", prefix, content)
	}

	return nil
}
