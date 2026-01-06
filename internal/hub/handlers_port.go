package hub

import (
	"encoding/json"
	"net/http"
	"strconv"

	"github.com/mbourmaud/hive/internal/port"
)

// handleListPorts handles GET /ports
func (h *Hub) handleListPorts(w http.ResponseWriter, r *http.Request) {
	leases := h.portRegistry.ListLeases()
	waiters := h.portRegistry.ListWaiters()

	h.jsonResponse(w, http.StatusOK, map[string]interface{}{
		"leases":  leases,
		"waiters": waiters,
	})
}

// handleGetPort handles GET /ports/{port}
func (h *Hub) handleGetPort(w http.ResponseWriter, r *http.Request) {
	portStr := r.PathValue("port")
	portNum, err := strconv.Atoi(portStr)
	if err != nil {
		h.jsonError(w, http.StatusBadRequest, "invalid port number")
		return
	}

	status := h.portRegistry.GetStatus(portNum)
	h.jsonResponse(w, http.StatusOK, status)
}

// handleAcquirePort handles POST /ports/acquire
func (h *Hub) handleAcquirePort(w http.ResponseWriter, r *http.Request) {
	var req port.AcquireRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		h.jsonError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	if req.Port == 0 {
		h.jsonError(w, http.StatusBadRequest, "port is required")
		return
	}
	if req.AgentID == "" {
		h.jsonError(w, http.StatusBadRequest, "agent_id is required")
		return
	}

	resp, err := h.portRegistry.Acquire(r.Context(), req)
	if err != nil {
		h.jsonError(w, http.StatusInternalServerError, err.Error())
		return
	}

	h.jsonResponse(w, http.StatusOK, resp)
}

// handleReleasePort handles POST /ports/release
func (h *Hub) handleReleasePort(w http.ResponseWriter, r *http.Request) {
	var req port.ReleaseRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		h.jsonError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	if req.Port == 0 {
		h.jsonError(w, http.StatusBadRequest, "port is required")
		return
	}
	if req.AgentID == "" {
		h.jsonError(w, http.StatusBadRequest, "agent_id is required")
		return
	}

	if err := h.portRegistry.Release(req); err != nil {
		h.jsonError(w, http.StatusBadRequest, err.Error())
		return
	}

	h.jsonResponse(w, http.StatusOK, map[string]string{"status": "released"})
}

// handleForceReleasePort handles POST /ports/{port}/force-release
func (h *Hub) handleForceReleasePort(w http.ResponseWriter, r *http.Request) {
	portStr := r.PathValue("port")
	portNum, err := strconv.Atoi(portStr)
	if err != nil {
		h.jsonError(w, http.StatusBadRequest, "invalid port number")
		return
	}

	var req port.ForceReleaseRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		// Allow empty body
		req = port.ForceReleaseRequest{}
	}
	req.Port = portNum

	if err := h.portRegistry.ForceRelease(req); err != nil {
		h.jsonError(w, http.StatusBadRequest, err.Error())
		return
	}

	h.jsonResponse(w, http.StatusOK, map[string]string{"status": "force_released"})
}

// handleStatus handles GET /status
func (h *Hub) handleStatus(w http.ResponseWriter, r *http.Request) {
	agents := h.agentManager.ListAgents()
	agentResponses := make([]AgentResponse, len(agents))
	for i, a := range agents {
		agentResponses[i] = agentToResponse(a)
	}

	tasks := h.taskManager.List("", "")
	solicitations := h.solicitationMgr.ListPending()
	portLeases := h.portRegistry.ListLeases()
	portWaiters := h.portRegistry.ListWaiters()

	h.jsonResponse(w, http.StatusOK, map[string]interface{}{
		"hub": map[string]interface{}{
			"port":   h.config.Port,
			"status": "running",
		},
		"agents": map[string]interface{}{
			"total":   len(agents),
			"running": h.agentManager.CountRunning(),
			"list":    agentResponses,
		},
		"tasks": map[string]interface{}{
			"total": len(tasks),
			"list":  tasks,
		},
		"solicitations": map[string]interface{}{
			"pending": len(solicitations),
			"counts":  h.solicitationMgr.Count(),
			"list":    solicitations,
		},
		"ports": map[string]interface{}{
			"leased":  len(portLeases),
			"waiting": len(portWaiters),
			"leases":  portLeases,
			"waiters": portWaiters,
		},
	})
}
