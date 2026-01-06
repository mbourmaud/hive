package monitor

import (
	"fmt"
	"strings"
	"time"

	"github.com/charmbracelet/bubbles/spinner"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
)

var (
	titleStyle = lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#FFD700")).
			MarginBottom(1)

	headerStyle = lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#7D56F4"))

	agentReadyStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#04B575"))

	agentBusyStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#FFAA00"))

	agentErrorStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#FF5555"))

	agentStoppedStyle = lipgloss.NewStyle().
				Foreground(lipgloss.Color("#888888"))

	solicitationStyle = lipgloss.NewStyle().
				Foreground(lipgloss.Color("#FF6B6B")).
				Bold(true)

	dimStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#666666"))

	boxStyle = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(lipgloss.Color("#444444")).
			Padding(0, 1)
)

type tickMsg time.Time
type dataMsg struct {
	agents        []Agent
	tasks         []Task
	solicitations []Solicitation
	err           error
}

type Model struct {
	client        *HubClient
	spinner       spinner.Model
	agents        []Agent
	tasks         []Task
	solicitations []Solicitation
	width         int
	height        int
	err           error
	connected     bool
}

func NewModel(hubURL string) Model {
	s := spinner.New()
	s.Spinner = spinner.Dot
	s.Style = lipgloss.NewStyle().Foreground(lipgloss.Color("#FFD700"))

	return Model{
		client:  NewHubClient(hubURL),
		spinner: s,
	}
}

func (m Model) Init() tea.Cmd {
	return tea.Batch(
		m.spinner.Tick,
		m.fetchData,
		m.tick(),
	)
}

func (m Model) tick() tea.Cmd {
	return tea.Tick(2*time.Second, func(t time.Time) tea.Msg {
		return tickMsg(t)
	})
}

func (m Model) fetchData() tea.Msg {
	agents, err := m.client.GetAgents()
	if err != nil {
		return dataMsg{err: err}
	}

	tasks, _ := m.client.GetTasks()
	solicitations, _ := m.client.GetSolicitations()

	return dataMsg{
		agents:        agents,
		tasks:         tasks,
		solicitations: solicitations,
	}
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.String() {
		case "q", "ctrl+c", "esc":
			m.client.Close()
			return m, tea.Quit
		case "r":
			return m, m.fetchData
		}

	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height

	case tickMsg:
		return m, tea.Batch(m.fetchData, m.tick())

	case dataMsg:
		if msg.err != nil {
			m.err = msg.err
			m.connected = false
		} else {
			m.err = nil
			m.connected = true
			m.agents = msg.agents
			m.tasks = msg.tasks
			m.solicitations = msg.solicitations
		}

	case spinner.TickMsg:
		var cmd tea.Cmd
		m.spinner, cmd = m.spinner.Update(msg)
		return m, cmd
	}

	return m, nil
}

func (m Model) View() string {
	var b strings.Builder

	b.WriteString(titleStyle.Render("🐝 Hive Monitor"))
	b.WriteString("\n\n")

	if m.err != nil {
		b.WriteString(agentErrorStyle.Render(fmt.Sprintf("❌ Connection error: %v", m.err)))
		b.WriteString("\n\n")
		b.WriteString(dimStyle.Render("Press 'r' to retry, 'q' to quit"))
		return b.String()
	}

	if !m.connected {
		b.WriteString(m.spinner.View())
		b.WriteString(" Connecting to Hub...")
		return b.String()
	}

	b.WriteString(m.renderAgents())
	b.WriteString("\n")
	b.WriteString(m.renderSolicitations())
	b.WriteString("\n")
	b.WriteString(m.renderTasks())
	b.WriteString("\n\n")
	b.WriteString(dimStyle.Render("Press 'r' to refresh, 'q' to quit"))

	return b.String()
}

func (m Model) renderAgents() string {
	var b strings.Builder

	b.WriteString(headerStyle.Render("AGENTS"))
	b.WriteString(fmt.Sprintf(" (%d)\n", len(m.agents)))

	if len(m.agents) == 0 {
		b.WriteString(dimStyle.Render("  No agents running\n"))
		return b.String()
	}

	for _, agent := range m.agents {
		statusIcon := m.getStatusIcon(agent.Status)
		statusStyle := m.getStatusStyle(agent.Status)

		line := fmt.Sprintf("  %s %s", statusIcon, statusStyle.Render(agent.Name))
		if agent.Specialty != "" {
			line += dimStyle.Render(fmt.Sprintf(" [%s]", agent.Specialty))
		}
		line += dimStyle.Render(fmt.Sprintf(" :%d %s", agent.Port, agent.Branch))
		b.WriteString(line + "\n")
	}

	return b.String()
}

func (m Model) renderSolicitations() string {
	var b strings.Builder

	pending := 0
	for _, s := range m.solicitations {
		if s.Status == "pending" {
			pending++
		}
	}

	if pending > 0 {
		b.WriteString(solicitationStyle.Render(fmt.Sprintf("⚠️  SOLICITATIONS (%d pending)\n", pending)))
		for _, s := range m.solicitations {
			if s.Status == "pending" {
				urgencyIcon := "○"
				if s.Urgency == "high" || s.Urgency == "critical" {
					urgencyIcon = "●"
				}
				b.WriteString(fmt.Sprintf("  %s [%s] %s: %s\n",
					urgencyIcon,
					s.Type,
					s.AgentName,
					truncate(s.Message, 50)))
			}
		}
	} else {
		b.WriteString(headerStyle.Render("SOLICITATIONS"))
		b.WriteString(dimStyle.Render(" (none pending)\n"))
	}

	return b.String()
}

func (m Model) renderTasks() string {
	var b strings.Builder

	activeTasks := 0
	for _, t := range m.tasks {
		if t.Status == "in_progress" || t.Status == "assigned" {
			activeTasks++
		}
	}

	b.WriteString(headerStyle.Render("TASKS"))
	b.WriteString(fmt.Sprintf(" (%d active)\n", activeTasks))

	if activeTasks == 0 {
		b.WriteString(dimStyle.Render("  No active tasks\n"))
		return b.String()
	}

	for _, t := range m.tasks {
		if t.Status == "in_progress" || t.Status == "assigned" {
			progress := ""
			if t.TotalSteps > 0 {
				progress = fmt.Sprintf(" [%d/%d]", t.CurrentStep, t.TotalSteps)
			}
			b.WriteString(fmt.Sprintf("  → %s%s %s\n",
				t.AgentName,
				progress,
				dimStyle.Render(truncate(t.Title, 40))))
		}
	}

	return b.String()
}

func (m Model) getStatusIcon(status string) string {
	switch status {
	case "ready":
		return "●"
	case "busy":
		return "◐"
	case "error":
		return "✗"
	case "stopped":
		return "○"
	default:
		return "?"
	}
}

func (m Model) getStatusStyle(status string) lipgloss.Style {
	switch status {
	case "ready":
		return agentReadyStyle
	case "busy":
		return agentBusyStyle
	case "error":
		return agentErrorStyle
	case "stopped":
		return agentStoppedStyle
	default:
		return dimStyle
	}
}

func truncate(s string, maxLen int) string {
	if len(s) <= maxLen {
		return s
	}
	return s[:maxLen-3] + "..."
}

func RunTUI(hubURL string) error {
	p := tea.NewProgram(NewModel(hubURL), tea.WithAltScreen())
	_, err := p.Run()
	return err
}
