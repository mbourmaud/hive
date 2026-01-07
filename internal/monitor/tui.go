package monitor

import (
	"fmt"
	"strings"
	"time"

	"github.com/charmbracelet/bubbles/spinner"
	"github.com/charmbracelet/bubbles/textinput"
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

	selectedStyle = lipgloss.NewStyle().
			Background(lipgloss.Color("#3b3b58")).
			Foreground(lipgloss.Color("#ffffff"))

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

	panelStyle = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(lipgloss.Color("#444444")).
			Padding(0, 1)

	userMsgStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#58a6ff"))

	assistantMsgStyle = lipgloss.NewStyle().
				Foreground(lipgloss.Color("#8b949e"))

	actionStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#238636"))
)

type tickMsg time.Time
type dataMsg struct {
	agents        []Agent
	tasks         []Task
	solicitations []Solicitation
	err           error
}
type conversationMsg struct {
	agentID  string
	messages []Message
	err      error
}

type viewMode int

const (
	viewList viewMode = iota
	viewDetail
	viewMessage
	viewSolicitation
	viewSolicitationResponse
)

type focusPanel int

const (
	focusAgents focusPanel = iota
	focusSolicitations
	focusTasks
)

type Model struct {
	client               *HubClient
	spinner              spinner.Model
	textInput            textinput.Model
	agents               []Agent
	tasks                []Task
	solicitations        []Solicitation
	conversation         []Message
	selectedIdx          int
	selectedAgent        *Agent
	selectedSolicitation *Solicitation
	selectedTask         *Task
	viewMode             viewMode
	focusPanel           focusPanel
	solicitationIdx      int
	taskIdx              int
	scrollOffset         int
	width                int
	height               int
	err                  error
	connected            bool
}

func NewModel(hubURL string) Model {
	s := spinner.New()
	s.Spinner = spinner.Dot
	s.Style = lipgloss.NewStyle().Foreground(lipgloss.Color("#FFD700"))

	ti := textinput.New()
	ti.Placeholder = "Type message to send..."
	ti.CharLimit = 500
	ti.Width = 50

	return Model{
		client:    NewHubClient(hubURL),
		spinner:   s,
		textInput: ti,
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

func (m Model) fetchConversation(agentID string) tea.Cmd {
	return func() tea.Msg {
		messages, err := m.client.GetConversation(agentID)
		return conversationMsg{agentID: agentID, messages: messages, err: err}
	}
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmds []tea.Cmd

	switch msg := msg.(type) {
	case tea.KeyMsg:
		if m.viewMode == viewMessage {
			return m.handleMessageInput(msg)
		}
		return m.handleKeyPress(msg)

	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height

	case tickMsg:
		cmds = append(cmds, m.fetchData, m.tick())
		if m.selectedAgent != nil {
			cmds = append(cmds, m.fetchConversation(m.selectedAgent.ID))
		}

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
			if m.selectedAgent != nil {
				for i, a := range m.agents {
					if a.ID == m.selectedAgent.ID {
						m.agents[i] = a
						m.selectedAgent = &m.agents[i]
						break
					}
				}
			}
		}

	case conversationMsg:
		if msg.err == nil && m.selectedAgent != nil && msg.agentID == m.selectedAgent.ID {
			m.conversation = msg.messages
		}

	case spinner.TickMsg:
		var cmd tea.Cmd
		m.spinner, cmd = m.spinner.Update(msg)
		cmds = append(cmds, cmd)
	}

	return m, tea.Batch(cmds...)
}

func (m Model) handleKeyPress(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch msg.String() {
	case "q", "ctrl+c":
		m.client.Close()
		return m, tea.Quit

	case "esc":
		if m.viewMode == viewDetail {
			m.viewMode = viewList
			m.selectedAgent = nil
			m.conversation = nil
		} else if m.viewMode == viewSolicitation || m.viewMode == viewSolicitationResponse {
			m.viewMode = viewList
			m.selectedSolicitation = nil
			m.textInput.Reset()
		}
		return m, nil

	case "tab":
		if m.viewMode == viewList {
			m.focusPanel = (m.focusPanel + 1) % 3
		}
		return m, nil

	case "up", "k":
		if m.viewMode == viewList {
			switch m.focusPanel {
			case focusAgents:
				if m.selectedIdx > 0 {
					m.selectedIdx--
				}
			case focusSolicitations:
				if m.solicitationIdx > 0 {
					m.solicitationIdx--
				}
			case focusTasks:
				if m.taskIdx > 0 {
					m.taskIdx--
				}
			}
		} else if m.viewMode == viewDetail && m.scrollOffset > 0 {
			m.scrollOffset--
		}

	case "down", "j":
		if m.viewMode == viewList {
			switch m.focusPanel {
			case focusAgents:
				if m.selectedIdx < len(m.agents)-1 {
					m.selectedIdx++
				}
			case focusSolicitations:
				pending := m.getPendingSolicitations()
				if m.solicitationIdx < len(pending)-1 {
					m.solicitationIdx++
				}
			case focusTasks:
				if m.taskIdx < len(m.tasks)-1 {
					m.taskIdx++
				}
			}
		} else if m.viewMode == viewDetail {
			m.scrollOffset++
		}

	case "enter":
		if m.viewMode == viewList {
			switch m.focusPanel {
			case focusAgents:
				if len(m.agents) > 0 {
					m.selectedAgent = &m.agents[m.selectedIdx]
					m.viewMode = viewDetail
					m.scrollOffset = 0
					return m, m.fetchConversation(m.selectedAgent.ID)
				}
			case focusSolicitations:
				pending := m.getPendingSolicitations()
				if len(pending) > 0 && m.solicitationIdx < len(pending) {
					m.selectedSolicitation = &pending[m.solicitationIdx]
					m.viewMode = viewSolicitation
				}
			case focusTasks:
				if len(m.tasks) > 0 && m.taskIdx < len(m.tasks) {
					m.selectedTask = &m.tasks[m.taskIdx]
				}
			}
		}

	case "r":
		return m, m.fetchData

	case "R":
		if m.viewMode == viewSolicitation && m.selectedSolicitation != nil {
			m.viewMode = viewSolicitationResponse
			m.textInput.Focus()
			return m, textinput.Blink
		}

	case "X":
		if m.viewMode == viewSolicitation && m.selectedSolicitation != nil {
			_ = m.client.DismissSolicitation(m.selectedSolicitation.ID)
			m.viewMode = viewList
			m.selectedSolicitation = nil
			return m, m.fetchData
		}

	case "s":
		if m.viewMode == viewList && m.focusPanel == focusTasks && m.selectedTask != nil {
			if m.selectedTask.Status == "pending" {
				_ = m.client.StartTask(m.selectedTask.ID)
				return m, m.fetchData
			}
		}

	case "c":
		if m.viewMode == viewList && m.focusPanel == focusTasks && m.selectedTask != nil {
			if m.selectedTask.Status == "in_progress" {
				_ = m.client.CompleteTask(m.selectedTask.ID)
				return m, m.fetchData
			}
		}

	case "x":
		if m.viewMode == viewList && m.focusPanel == focusTasks && m.selectedTask != nil {
			_ = m.client.CancelTask(m.selectedTask.ID)
			return m, m.fetchData
		}

	case "K":
		if m.selectedAgent != nil {
			_ = m.client.KillAgent(m.selectedAgent.ID)
			return m, m.fetchData
		}

	case "D":
		if m.selectedAgent != nil {
			_ = m.client.DestroyAgent(m.selectedAgent.ID)
			m.viewMode = viewList
			m.selectedAgent = nil
			return m, m.fetchData
		}

	case "m":
		if m.selectedAgent != nil {
			m.viewMode = viewMessage
			m.textInput.Focus()
			return m, textinput.Blink
		}
	}

	return m, nil
}

func (m Model) getPendingSolicitations() []Solicitation {
	var pending []Solicitation
	for _, s := range m.solicitations {
		if s.Status == "pending" {
			pending = append(pending, s)
		}
	}
	return pending
}

func (m Model) handleMessageInput(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch msg.String() {
	case "esc":
		if m.viewMode == viewSolicitationResponse {
			m.viewMode = viewSolicitation
		} else {
			m.viewMode = viewDetail
		}
		m.textInput.Reset()
		return m, nil

	case "enter":
		content := m.textInput.Value()
		if content != "" {
			if m.viewMode == viewSolicitationResponse && m.selectedSolicitation != nil {
				_ = m.client.RespondSolicitation(m.selectedSolicitation.ID, content)
				m.textInput.Reset()
				m.viewMode = viewList
				m.selectedSolicitation = nil
				return m, m.fetchData
			} else if m.selectedAgent != nil {
				_ = m.client.SendMessage(m.selectedAgent.ID, content)
				m.textInput.Reset()
				m.viewMode = viewDetail
				return m, m.fetchConversation(m.selectedAgent.ID)
			}
		}
		return m, nil
	}

	var cmd tea.Cmd
	m.textInput, cmd = m.textInput.Update(msg)
	return m, cmd
}

func (m Model) View() string {
	if m.err != nil {
		return m.renderError()
	}

	if !m.connected {
		return m.renderConnecting()
	}

	if m.viewMode == viewMessage {
		return m.renderMessageInput()
	}

	if m.viewMode == viewSolicitationResponse {
		return m.renderSolicitationResponseInput()
	}

	if m.viewMode == viewSolicitation {
		return m.renderSolicitationDetail()
	}

	if m.viewMode == viewDetail {
		return m.renderSplitView()
	}

	return m.renderListView()
}

func (m Model) renderSolicitationDetail() string {
	var b strings.Builder
	b.WriteString(titleStyle.Render("🐝 Hive Monitor"))
	b.WriteString("\n\n")

	if m.selectedSolicitation == nil {
		b.WriteString(dimStyle.Render("No solicitation selected"))
		return b.String()
	}

	sol := m.selectedSolicitation
	b.WriteString(solicitationStyle.Render("SOLICITATION"))
	b.WriteString("\n")
	b.WriteString(strings.Repeat("─", 50))
	b.WriteString("\n\n")

	b.WriteString(fmt.Sprintf("From:     %s\n", headerStyle.Render(sol.AgentName)))
	b.WriteString(fmt.Sprintf("Type:     %s\n", sol.Type))
	b.WriteString(fmt.Sprintf("Urgency:  %s\n", sol.Urgency))
	b.WriteString("\n")

	b.WriteString(headerStyle.Render("MESSAGE"))
	b.WriteString("\n")
	b.WriteString(strings.Repeat("─", 50))
	b.WriteString("\n")
	b.WriteString(sol.Message)
	b.WriteString("\n\n")

	b.WriteString(dimStyle.Render("R: Respond  X: Dismiss  Esc: Back  q: Quit"))
	return b.String()
}

func (m Model) renderSolicitationResponseInput() string {
	var b strings.Builder
	b.WriteString(titleStyle.Render("🐝 Hive Monitor"))
	b.WriteString("\n\n")
	b.WriteString(fmt.Sprintf("Respond to %s:\n\n", headerStyle.Render(m.selectedSolicitation.AgentName)))
	b.WriteString(dimStyle.Render(fmt.Sprintf("[%s] %s\n\n", m.selectedSolicitation.Type, truncate(m.selectedSolicitation.Message, 60))))
	b.WriteString(m.textInput.View())
	b.WriteString("\n\n")
	b.WriteString(dimStyle.Render("Enter: Send  Esc: Cancel"))
	return b.String()
}

func (m Model) renderError() string {
	var b strings.Builder
	b.WriteString(titleStyle.Render("🐝 Hive Monitor"))
	b.WriteString("\n\n")
	b.WriteString(agentErrorStyle.Render(fmt.Sprintf("❌ Connection error: %v", m.err)))
	b.WriteString("\n\n")
	b.WriteString(dimStyle.Render("Press 'r' to retry, 'q' to quit"))
	return b.String()
}

func (m Model) renderConnecting() string {
	var b strings.Builder
	b.WriteString(titleStyle.Render("🐝 Hive Monitor"))
	b.WriteString("\n\n")
	b.WriteString(m.spinner.View())
	b.WriteString(" Connecting to Hub...")
	return b.String()
}

func (m Model) renderMessageInput() string {
	var b strings.Builder
	b.WriteString(titleStyle.Render("🐝 Hive Monitor"))
	b.WriteString("\n\n")
	b.WriteString(fmt.Sprintf("Send message to %s:\n\n", headerStyle.Render(m.selectedAgent.Name)))
	b.WriteString(m.textInput.View())
	b.WriteString("\n\n")
	b.WriteString(dimStyle.Render("Enter: Send  Esc: Cancel"))
	return b.String()
}

func (m Model) renderListView() string {
	var b strings.Builder

	b.WriteString(titleStyle.Render("🐝 Hive Monitor"))
	b.WriteString("\n\n")

	b.WriteString(m.renderAgentsListWithFocus())
	b.WriteString("\n")
	b.WriteString(m.renderSolicitationsWithFocus())
	b.WriteString("\n")
	b.WriteString(m.renderTasksWithFocus())
	b.WriteString("\n\n")

	help := "Tab: Switch Panel  ↑↓: Navigate  Enter: Select  r: Refresh  q: Quit"
	if m.focusPanel == focusTasks {
		help = "Tab: Switch  ↑↓: Navigate  s: Start  c: Complete  x: Cancel  q: Quit"
	}
	b.WriteString(dimStyle.Render(help))

	return b.String()
}

func (m Model) renderAgentsListWithFocus() string {
	var b strings.Builder

	header := "AGENTS"
	if m.focusPanel == focusAgents {
		header = "▶ " + header
	}
	b.WriteString(headerStyle.Render(header))
	b.WriteString(fmt.Sprintf(" (%d)\n", len(m.agents)))

	if len(m.agents) == 0 {
		b.WriteString(dimStyle.Render("  No agents running\n"))
		return b.String()
	}

	for i, agent := range m.agents {
		line := m.formatAgentLine(agent)
		if m.focusPanel == focusAgents && i == m.selectedIdx {
			line = selectedStyle.Render(line)
		}
		b.WriteString(line + "\n")
	}

	return b.String()
}

func (m Model) renderSolicitationsWithFocus() string {
	var b strings.Builder

	pending := m.getPendingSolicitations()

	header := "SOLICITATIONS"
	if m.focusPanel == focusSolicitations {
		header = "▶ " + header
	}

	if len(pending) > 0 {
		b.WriteString(solicitationStyle.Render(fmt.Sprintf("⚠️  %s (%d pending)\n", header, len(pending))))
		for i, s := range pending {
			urgencyIcon := "○"
			if s.Urgency == "high" || s.Urgency == "critical" {
				urgencyIcon = "●"
			}
			line := fmt.Sprintf("  %s [%s] %s: %s",
				urgencyIcon,
				s.Type,
				s.AgentName,
				truncate(s.Message, 50))
			if m.focusPanel == focusSolicitations && i == m.solicitationIdx {
				line = selectedStyle.Render(line)
			}
			b.WriteString(line + "\n")
		}
	} else {
		b.WriteString(headerStyle.Render(header))
		b.WriteString(dimStyle.Render(" (none pending)\n"))
	}

	return b.String()
}

func (m Model) renderTasksWithFocus() string {
	var b strings.Builder

	header := "TASKS"
	if m.focusPanel == focusTasks {
		header = "▶ " + header
	}

	b.WriteString(headerStyle.Render(header))
	b.WriteString(fmt.Sprintf(" (%d)\n", len(m.tasks)))

	if len(m.tasks) == 0 {
		b.WriteString(dimStyle.Render("  No tasks\n"))
		return b.String()
	}

	for i, t := range m.tasks {
		statusIcon := "○"
		switch t.Status {
		case "in_progress":
			statusIcon = "◐"
		case "completed":
			statusIcon = "●"
		case "failed":
			statusIcon = "✗"
		}
		progress := ""
		totalSteps := 0
		title := "Untitled"
		if t.Plan != nil {
			totalSteps = len(t.Plan.Steps)
			title = t.Plan.Title
		}
		if totalSteps > 0 {
			progress = fmt.Sprintf(" [%d/%d]", t.CurrentStep, totalSteps)
		}
		line := fmt.Sprintf("  %s %s%s %s",
			statusIcon,
			t.AgentName,
			progress,
			dimStyle.Render(truncate(title, 40)))
		if m.focusPanel == focusTasks && i == m.taskIdx {
			line = selectedStyle.Render(line)
			m.selectedTask = &m.tasks[i]
		}
		b.WriteString(line + "\n")
	}

	return b.String()
}

func (m Model) renderSplitView() string {
	leftWidth := 35
	rightWidth := m.width - leftWidth - 5
	if rightWidth < 40 {
		rightWidth = 40
	}

	left := m.renderAgentsListCompact(leftWidth)
	right := m.renderAgentDetail(rightWidth)

	leftPanel := panelStyle.Width(leftWidth).Render(left)
	rightPanel := panelStyle.Width(rightWidth).Render(right)

	split := lipgloss.JoinHorizontal(lipgloss.Top, leftPanel, " ", rightPanel)

	var b strings.Builder
	b.WriteString(titleStyle.Render("🐝 Hive Monitor"))
	b.WriteString("\n\n")
	b.WriteString(split)
	b.WriteString("\n\n")
	b.WriteString(dimStyle.Render("↑↓: Scroll  K: Kill  D: Destroy  m: Message  Esc: Back  q: Quit"))

	return b.String()
}

func (m Model) renderAgentsList() string {
	var b strings.Builder

	b.WriteString(headerStyle.Render("AGENTS"))
	b.WriteString(fmt.Sprintf(" (%d)\n", len(m.agents)))

	if len(m.agents) == 0 {
		b.WriteString(dimStyle.Render("  No agents running\n"))
		return b.String()
	}

	for i, agent := range m.agents {
		line := m.formatAgentLine(agent)
		if i == m.selectedIdx {
			line = selectedStyle.Render(line)
		}
		b.WriteString(line + "\n")
	}

	return b.String()
}

func (m Model) renderAgentsListCompact(width int) string {
	var b strings.Builder

	b.WriteString(headerStyle.Render("AGENTS"))
	b.WriteString(fmt.Sprintf(" (%d)\n\n", len(m.agents)))

	for _, agent := range m.agents {
		statusIcon := m.getStatusIcon(agent.Status)
		statusStyle := m.getStatusStyle(agent.Status)

		name := agent.Name
		if len(name) > width-6 {
			name = name[:width-9] + "..."
		}

		line := fmt.Sprintf("  %s %s", statusIcon, statusStyle.Render(name))

		if m.selectedAgent != nil && agent.ID == m.selectedAgent.ID {
			line = selectedStyle.Render(line)
		}
		b.WriteString(line + "\n")
	}

	return b.String()
}

func (m Model) renderAgentDetail(width int) string {
	if m.selectedAgent == nil {
		return dimStyle.Render("Select an agent")
	}

	var b strings.Builder
	agent := m.selectedAgent

	b.WriteString(headerStyle.Render(agent.Name))
	if agent.Specialty != "" {
		b.WriteString(dimStyle.Render(fmt.Sprintf(" [%s]", agent.Specialty)))
	}
	b.WriteString("\n")
	b.WriteString(strings.Repeat("─", width-2))
	b.WriteString("\n\n")

	statusStyle := m.getStatusStyle(agent.Status)
	b.WriteString(fmt.Sprintf("Status:  %s\n", statusStyle.Render(agent.Status)))
	b.WriteString(fmt.Sprintf("Port:    %d\n", agent.Port))
	b.WriteString(fmt.Sprintf("Branch:  %s\n", agent.Branch))
	b.WriteString("\n")

	b.WriteString(headerStyle.Render("CONVERSATION"))
	b.WriteString("\n")
	b.WriteString(strings.Repeat("─", width-2))
	b.WriteString("\n")

	if len(m.conversation) == 0 {
		b.WriteString(dimStyle.Render("No messages yet\n"))
	} else {
		maxMessages := 10
		start := 0
		if len(m.conversation) > maxMessages {
			start = len(m.conversation) - maxMessages
		}

		for _, msg := range m.conversation[start:] {
			var style lipgloss.Style
			var prefix string
			if msg.Role == "user" {
				style = userMsgStyle
				prefix = "▶ "
			} else {
				style = assistantMsgStyle
				prefix = "◀ "
			}

			content := msg.Content
			if len(content) > width-10 {
				content = content[:width-13] + "..."
			}
			content = strings.ReplaceAll(content, "\n", " ")

			b.WriteString(style.Render(prefix + content))
			b.WriteString("\n")
		}
	}

	return b.String()
}

func (m Model) formatAgentLine(agent Agent) string {
	statusIcon := m.getStatusIcon(agent.Status)
	statusStyle := m.getStatusStyle(agent.Status)

	line := fmt.Sprintf("  %s %s", statusIcon, statusStyle.Render(agent.Name))
	if agent.Specialty != "" {
		line += dimStyle.Render(fmt.Sprintf(" [%s]", agent.Specialty))
	}
	line += dimStyle.Render(fmt.Sprintf(" :%d %s", agent.Port, agent.Branch))
	return line
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
			totalSteps := 0
			title := "Untitled"
			if t.Plan != nil {
				totalSteps = len(t.Plan.Steps)
				title = t.Plan.Title
			}
			if totalSteps > 0 {
				progress = fmt.Sprintf(" [%d/%d]", t.CurrentStep, totalSteps)
			}
			b.WriteString(fmt.Sprintf("  → %s%s %s\n",
				t.AgentName,
				progress,
				dimStyle.Render(truncate(title, 40))))
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
