package hub

import (
	"encoding/json"
	"net/http"

	"github.com/mbourmaud/hive/internal/agent"
)

// SpawnRequest represents a request to spawn a new agent.
type SpawnRequest struct {
	Name       string `json:"name"`
	Branch     string `json:"branch,omitempty"`
	BaseBranch string `json:"base_branch,omitempty"`
	Specialty  string `json:"specialty,omitempty"`
	Sandbox    bool   `json:"sandbox,omitempty"`
}

// MessageRequest represents a request to send a message.
type MessageRequest struct {
	Content string `json:"content"`
}

// AgentResponse represents an agent in API responses.
type AgentResponse struct {
	ID           string `json:"id"`
	Name         string `json:"name"`
	WorktreePath string `json:"worktree_path"`
	Branch       string `json:"branch"`
	Port         int    `json:"port"`
	Status       string `json:"status"`
	Specialty    string `json:"specialty,omitempty"`
	CreatedAt    string `json:"created_at"`
	Error        string `json:"error,omitempty"`
}

func agentToResponse(a *agent.Agent) AgentResponse {
	return AgentResponse{
		ID:           a.ID,
		Name:         a.Name,
		WorktreePath: a.WorktreePath,
		Branch:       a.Branch,
		Port:         a.Port,
		Status:       string(a.Status),
		Specialty:    a.Specialty,
		CreatedAt:    a.CreatedAt.Format("2006-01-02T15:04:05Z07:00"),
		Error:        a.Error,
	}
}

// handleListAgents handles GET /agents
func (h *Hub) handleListAgents(w http.ResponseWriter, r *http.Request) {
	agents := h.agentManager.ListAgents()

	responses := make([]AgentResponse, len(agents))
	for i, a := range agents {
		responses[i] = agentToResponse(a)
	}

	h.jsonResponse(w, http.StatusOK, responses)
}

// handleSpawnAgent handles POST /agents
func (h *Hub) handleSpawnAgent(w http.ResponseWriter, r *http.Request) {
	var req SpawnRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		h.jsonError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	if req.Name == "" {
		h.jsonError(w, http.StatusBadRequest, "name is required")
		return
	}

	// Use hub config defaults
	sandbox := req.Sandbox
	if !sandbox && h.config.Sandbox {
		sandbox = true
	}

	opts := agent.SpawnOptions{
		Name:       req.Name,
		RepoPath:   h.config.RepoPath,
		Branch:     req.Branch,
		BaseBranch: req.BaseBranch,
		Specialty:  req.Specialty,
		Sandbox:    sandbox,
	}

	a, err := h.agentManager.SpawnAgent(r.Context(), opts)
	if err != nil {
		h.jsonError(w, http.StatusInternalServerError, err.Error())
		return
	}

	// Emit event
	h.eventHub.Broadcast(Event{
		Type: EventAgentSpawned,
		Data: agentToResponse(a),
	})

	h.jsonResponse(w, http.StatusCreated, agentToResponse(a))
}

// handleGetAgent handles GET /agents/{id}
func (h *Hub) handleGetAgent(w http.ResponseWriter, r *http.Request) {
	id := r.PathValue("id")

	// Try by ID first, then by name
	a, err := h.agentManager.GetAgent(id)
	if err != nil {
		a, err = h.agentManager.GetAgentByName(id)
		if err != nil {
			h.jsonError(w, http.StatusNotFound, "agent not found")
			return
		}
	}

	// Refresh status from AgentAPI
	h.agentManager.RefreshStatus(r.Context(), a.ID)

	h.jsonResponse(w, http.StatusOK, agentToResponse(a))
}

// handleStopAgent handles DELETE /agents/{id}
func (h *Hub) handleStopAgent(w http.ResponseWriter, r *http.Request) {
	id := r.PathValue("id")

	// Try by ID first, then by name
	a, err := h.agentManager.GetAgent(id)
	if err != nil {
		a, err = h.agentManager.GetAgentByName(id)
		if err != nil {
			h.jsonError(w, http.StatusNotFound, "agent not found")
			return
		}
	}

	if err := h.agentManager.StopAgent(r.Context(), a.ID); err != nil {
		h.jsonError(w, http.StatusInternalServerError, err.Error())
		return
	}

	// Emit event
	h.eventHub.Broadcast(Event{
		Type: EventAgentStopped,
		Data: map[string]string{"id": a.ID, "name": a.Name},
	})

	h.jsonResponse(w, http.StatusOK, map[string]string{"status": "stopped"})
}

// handleDestroyAgent handles DELETE /agents/{id}/destroy
func (h *Hub) handleDestroyAgent(w http.ResponseWriter, r *http.Request) {
	id := r.PathValue("id")

	// Try by ID first, then by name
	a, err := h.agentManager.GetAgent(id)
	if err != nil {
		a, err = h.agentManager.GetAgentByName(id)
		if err != nil {
			h.jsonError(w, http.StatusNotFound, "agent not found")
			return
		}
	}

	agentID := a.ID
	agentName := a.Name

	if err := h.agentManager.DestroyAgent(r.Context(), agentID); err != nil {
		h.jsonError(w, http.StatusInternalServerError, err.Error())
		return
	}

	// Emit event
	h.eventHub.Broadcast(Event{
		Type: EventAgentDestroyed,
		Data: map[string]string{"id": agentID, "name": agentName},
	})

	h.jsonResponse(w, http.StatusOK, map[string]string{"status": "destroyed"})
}

// handleSendMessage handles POST /agents/{id}/message
func (h *Hub) handleSendMessage(w http.ResponseWriter, r *http.Request) {
	id := r.PathValue("id")

	var req MessageRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		h.jsonError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	if req.Content == "" {
		h.jsonError(w, http.StatusBadRequest, "content is required")
		return
	}

	// Try by ID first, then by name
	a, err := h.agentManager.GetAgent(id)
	if err != nil {
		a, err = h.agentManager.GetAgentByName(id)
		if err != nil {
			h.jsonError(w, http.StatusNotFound, "agent not found")
			return
		}
	}

	if err := h.agentManager.SendMessage(r.Context(), a.ID, req.Content); err != nil {
		h.jsonError(w, http.StatusInternalServerError, err.Error())
		return
	}

	// Emit event
	h.eventHub.Broadcast(Event{
		Type: EventMessageSent,
		Data: map[string]string{"agent_id": a.ID, "content": req.Content},
	})

	h.jsonResponse(w, http.StatusOK, map[string]string{"status": "sent"})
}

// handleGetConversation handles GET /agents/{id}/conversation
func (h *Hub) handleGetConversation(w http.ResponseWriter, r *http.Request) {
	id := r.PathValue("id")

	// Try by ID first, then by name
	a, err := h.agentManager.GetAgent(id)
	if err != nil {
		a, err = h.agentManager.GetAgentByName(id)
		if err != nil {
			h.jsonError(w, http.StatusNotFound, "agent not found")
			return
		}
	}

	messages, err := h.agentManager.GetConversation(r.Context(), a.ID)
	if err != nil {
		h.jsonError(w, http.StatusInternalServerError, err.Error())
		return
	}

	h.jsonResponse(w, http.StatusOK, messages)
}

// handleGetAgentStatus handles GET /agents/{id}/status
func (h *Hub) handleGetAgentStatus(w http.ResponseWriter, r *http.Request) {
	id := r.PathValue("id")

	// Try by ID first, then by name
	a, err := h.agentManager.GetAgent(id)
	if err != nil {
		a, err = h.agentManager.GetAgentByName(id)
		if err != nil {
			h.jsonError(w, http.StatusNotFound, "agent not found")
			return
		}
	}

	// Refresh status
	h.agentManager.RefreshStatus(r.Context(), a.ID)

	h.jsonResponse(w, http.StatusOK, map[string]string{
		"id":     a.ID,
		"name":   a.Name,
		"status": string(a.Status),
	})
}

// handleHealth handles GET /health
func (h *Hub) handleHealth(w http.ResponseWriter, r *http.Request) {
	h.jsonResponse(w, http.StatusOK, map[string]interface{}{
		"status":         "ok",
		"agents_total":   h.agentManager.Count(),
		"agents_running": h.agentManager.CountRunning(),
	})
}
