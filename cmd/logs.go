package cmd

import (
	"context"
	"fmt"
	"os"
	"os/exec"
	"time"

	"github.com/redis/go-redis/v9"
	"github.com/spf13/cobra"
)

var (
	logsFollow   bool
	logsTail     int
	logsActivity bool
)

var logsCmd = &cobra.Command{
	Use:   "logs [id]",
	Short: "View container or activity logs",
	Long: `View logs for a specific agent container or Claude activity.

Examples:
  hive logs queen           # View queen container logs
  hive logs 1               # View worker 1 container logs
  hive logs queen -f        # Follow logs in real-time
  hive logs 1 --tail 50     # Show last 50 lines
  hive logs --activity      # View all Claude activity from Redis
  hive logs --activity 1    # View Claude activity for worker 1
  hive logs --activity -f   # Follow activity in real-time`,
	Args: cobra.MaximumNArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		if logsActivity {
			return showActivityLogs(args)
		}

		// Default: container logs (requires id)
		if len(args) == 0 {
			return fmt.Errorf("container id required (e.g., 'hive logs queen' or 'hive logs 1')")
		}

		id := args[0]
		containerName := mapAgentID(id)

		fmt.Printf("Logs for %s:\n\n", containerName)

		// Build docker logs command
		dockerArgs := []string{"logs"}

		if logsFollow {
			dockerArgs = append(dockerArgs, "-f")
		}

		if logsTail > 0 {
			dockerArgs = append(dockerArgs, "--tail", fmt.Sprintf("%d", logsTail))
		}

		dockerArgs = append(dockerArgs, containerName)

		dockerCmd := exec.Command("docker", dockerArgs...)
		dockerCmd.Stdout = os.Stdout
		dockerCmd.Stderr = os.Stderr

		if err := dockerCmd.Run(); err != nil {
			return fmt.Errorf("failed to get logs for %s: %w", containerName, err)
		}

		return nil
	},
}

func showActivityLogs(args []string) error {
	ctx := context.Background()

	// Connect to Redis
	rdb := redis.NewClient(&redis.Options{
		Addr: "localhost:6380",
	})
	defer rdb.Close()

	if err := rdb.Ping(ctx).Err(); err != nil {
		return fmt.Errorf("failed to connect to Redis: %w\nMake sure hive is running", err)
	}

	// Determine stream key
	var streamKey string
	if len(args) > 0 {
		agentID := args[0]
		if agentID == "queen" || agentID == "q" || agentID == "0" {
			streamKey = "hive:logs:queen"
		} else {
			streamKey = fmt.Sprintf("hive:logs:drone-%s", agentID)
		}
		fmt.Printf("📋 Activity logs for %s\n\n", streamKey)
	} else {
		streamKey = "hive:logs:all"
		fmt.Printf("📋 All activity logs\n\n")
	}

	if logsFollow {
		// Follow mode: stream new entries
		fmt.Println("Following activity... (Ctrl+C to stop)")
		lastID := "$" // Start from new entries

		for {
			streams, err := rdb.XRead(ctx, &redis.XReadArgs{
				Streams: []string{streamKey, lastID},
				Block:   5 * time.Second,
				Count:   10,
			}).Result()

			if err == redis.Nil {
				continue
			}
			if err != nil {
				return fmt.Errorf("failed to read stream: %w", err)
			}

			for _, stream := range streams {
				for _, msg := range stream.Messages {
					printActivityEntry(msg)
					lastID = msg.ID
				}
			}
		}
	} else {
		// Read last N entries
		entries, err := rdb.XRevRange(ctx, streamKey, "+", "-").Result()
		if err != nil {
			return fmt.Errorf("failed to read stream: %w", err)
		}

		if len(entries) == 0 {
			fmt.Println("No activity logs found.")
			return nil
		}

		// Reverse to show oldest first, limit to tail
		start := 0
		if len(entries) > logsTail {
			start = len(entries) - logsTail
		}

		for i := len(entries) - 1; i >= start; i-- {
			printActivityEntry(entries[i])
		}
	}

	return nil
}

func printActivityEntry(msg redis.XMessage) {
	timestamp := msg.Values["timestamp"]
	agent := msg.Values["agent"]
	event := msg.Values["event"]
	content := msg.Values["content"]

	// Color based on event type
	var icon string
	switch event {
	case "task_start":
		icon = "🚀"
	case "claude_response":
		icon = "💬"
	case "tool_call":
		icon = "🔧"
	case "tool_result":
		icon = "✓"
	case "tool_error":
		icon = "❌"
	case "task_complete":
		icon = "✅"
	case "task_failed":
		icon = "💥"
	default:
		icon = "•"
	}

	// Truncate content for display
	contentStr := fmt.Sprintf("%v", content)
	if len(contentStr) > 100 {
		contentStr = contentStr[:100] + "..."
	}

	fmt.Printf("[%s] %s %s %s: %s\n", timestamp, icon, agent, event, contentStr)
}

func init() {
	rootCmd.AddCommand(logsCmd)
	logsCmd.Flags().BoolVarP(&logsFollow, "follow", "f", false, "Follow log output")
	logsCmd.Flags().IntVar(&logsTail, "tail", 100, "Number of lines to show from the end")
	logsCmd.Flags().BoolVar(&logsActivity, "activity", false, "Show Claude activity logs from Redis")
}
