package hub

import (
	"encoding/json"
	"net/http"

	"github.com/mbourmaud/hive/internal/solicitation"
)

// handleListSolicitations handles GET /solicitations
func (h *Hub) handleListSolicitations(w http.ResponseWriter, r *http.Request) {
	filter := solicitation.ListFilter{
		AgentID: r.URL.Query().Get("agent_id"),
		Type:    solicitation.Type(r.URL.Query().Get("type")),
		Urgency: solicitation.Urgency(r.URL.Query().Get("urgency")),
		Status:  solicitation.Status(r.URL.Query().Get("status")),
	}

	// Default to pending if no status specified
	if filter.Status == "" {
		filter.Status = solicitation.StatusPending
	}

	sols := h.solicitationMgr.List(filter)

	h.jsonResponse(w, http.StatusOK, sols)
}

// handleCreateSolicitation handles POST /solicitations
func (h *Hub) handleCreateSolicitation(w http.ResponseWriter, r *http.Request) {
	var req solicitation.CreateRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		h.jsonError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	sol, err := h.solicitationMgr.Create(req)
	if err != nil {
		h.jsonError(w, http.StatusBadRequest, err.Error())
		return
	}

	h.jsonResponse(w, http.StatusCreated, sol)
}

// handleGetSolicitation handles GET /solicitations/{id}
func (h *Hub) handleGetSolicitation(w http.ResponseWriter, r *http.Request) {
	id := r.PathValue("id")

	sol, err := h.solicitationMgr.Get(id)
	if err != nil {
		h.jsonError(w, http.StatusNotFound, err.Error())
		return
	}

	h.jsonResponse(w, http.StatusOK, sol)
}

// handleRespondSolicitation handles POST /solicitations/{id}/respond
func (h *Hub) handleRespondSolicitation(w http.ResponseWriter, r *http.Request) {
	id := r.PathValue("id")

	var req solicitation.RespondRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		h.jsonError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	if err := h.solicitationMgr.Respond(id, req); err != nil {
		h.jsonError(w, http.StatusBadRequest, err.Error())
		return
	}

	sol, _ := h.solicitationMgr.Get(id)
	h.jsonResponse(w, http.StatusOK, sol)
}

// handleDismissSolicitation handles POST /solicitations/{id}/dismiss
func (h *Hub) handleDismissSolicitation(w http.ResponseWriter, r *http.Request) {
	id := r.PathValue("id")

	var req solicitation.DismissRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		// Allow empty body
		req = solicitation.DismissRequest{}
	}

	if err := h.solicitationMgr.Dismiss(id, req); err != nil {
		h.jsonError(w, http.StatusBadRequest, err.Error())
		return
	}

	sol, _ := h.solicitationMgr.Get(id)
	h.jsonResponse(w, http.StatusOK, sol)
}
