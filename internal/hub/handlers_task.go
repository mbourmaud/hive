package hub

import (
	"encoding/json"
	"net/http"
	"strconv"

	"github.com/mbourmaud/hive/internal/task"
)

// handleListTasks handles GET /tasks
func (h *Hub) handleListTasks(w http.ResponseWriter, r *http.Request) {
	agentID := r.URL.Query().Get("agent_id")
	status := r.URL.Query().Get("status")

	tasks := h.taskManager.List(agentID, task.TaskStatus(status))

	h.jsonResponse(w, http.StatusOK, tasks)
}

// handleCreateTask handles POST /tasks
func (h *Hub) handleCreateTask(w http.ResponseWriter, r *http.Request) {
	var req task.CreateTaskRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		h.jsonError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	t, err := h.taskManager.Create(req)
	if err != nil {
		h.jsonError(w, http.StatusBadRequest, err.Error())
		return
	}

	h.jsonResponse(w, http.StatusCreated, t)
}

// handleGetTask handles GET /tasks/{id}
func (h *Hub) handleGetTask(w http.ResponseWriter, r *http.Request) {
	id := r.PathValue("id")

	t, err := h.taskManager.Get(id)
	if err != nil {
		h.jsonError(w, http.StatusNotFound, err.Error())
		return
	}

	h.jsonResponse(w, http.StatusOK, t)
}

// handleStartTask handles POST /tasks/{id}/start
func (h *Hub) handleStartTask(w http.ResponseWriter, r *http.Request) {
	id := r.PathValue("id")

	if err := h.taskManager.Start(id); err != nil {
		h.jsonError(w, http.StatusBadRequest, err.Error())
		return
	}

	t, _ := h.taskManager.Get(id)
	h.jsonResponse(w, http.StatusOK, t)
}

// handleUpdateStep handles PUT /tasks/{id}/steps/{step}
func (h *Hub) handleUpdateStep(w http.ResponseWriter, r *http.Request) {
	taskID := r.PathValue("id")
	stepStr := r.PathValue("step")

	stepID, err := strconv.Atoi(stepStr)
	if err != nil {
		h.jsonError(w, http.StatusBadRequest, "invalid step ID")
		return
	}

	var req task.UpdateStepRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		h.jsonError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	if err := h.taskManager.UpdateStep(taskID, stepID, req); err != nil {
		h.jsonError(w, http.StatusBadRequest, err.Error())
		return
	}

	t, _ := h.taskManager.Get(taskID)
	h.jsonResponse(w, http.StatusOK, t)
}

// handleCompleteTask handles POST /tasks/{id}/complete
func (h *Hub) handleCompleteTask(w http.ResponseWriter, r *http.Request) {
	id := r.PathValue("id")

	var req task.CompleteTaskRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		// Allow empty body
		req = task.CompleteTaskRequest{}
	}

	if err := h.taskManager.Complete(id, req); err != nil {
		h.jsonError(w, http.StatusBadRequest, err.Error())
		return
	}

	t, _ := h.taskManager.Get(id)
	h.jsonResponse(w, http.StatusOK, t)
}

// handleFailTask handles POST /tasks/{id}/fail
func (h *Hub) handleFailTask(w http.ResponseWriter, r *http.Request) {
	id := r.PathValue("id")

	var req task.FailTaskRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		h.jsonError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	if err := h.taskManager.Fail(id, req); err != nil {
		h.jsonError(w, http.StatusBadRequest, err.Error())
		return
	}

	t, _ := h.taskManager.Get(id)
	h.jsonResponse(w, http.StatusOK, t)
}

// handleCancelTask handles DELETE /tasks/{id}
func (h *Hub) handleCancelTask(w http.ResponseWriter, r *http.Request) {
	id := r.PathValue("id")
	reason := r.URL.Query().Get("reason")
	if reason == "" {
		reason = "cancelled by user"
	}

	if err := h.taskManager.Cancel(id, reason); err != nil {
		h.jsonError(w, http.StatusBadRequest, err.Error())
		return
	}

	h.jsonResponse(w, http.StatusOK, map[string]string{"status": "cancelled"})
}
