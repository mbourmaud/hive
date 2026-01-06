package hub

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"sync"
)

// EventType represents the type of event.
type EventType string

const (
	// Agent events
	EventAgentSpawned    EventType = "agent.spawned"
	EventAgentStopped    EventType = "agent.stopped"
	EventAgentDestroyed  EventType = "agent.destroyed"
	EventAgentStatus     EventType = "agent.status"
	EventMessageSent     EventType = "message.sent"
	EventMessageReceived EventType = "message.received"

	// Task events
	EventTaskCreated   EventType = "task.created"
	EventTaskStarted   EventType = "task.started"
	EventTaskProgress  EventType = "task.progress"
	EventTaskCompleted EventType = "task.completed"
	EventTaskFailed    EventType = "task.failed"
	EventTaskCancelled EventType = "task.cancelled"

	// Solicitation events
	EventSolicitationNew       EventType = "solicitation.new"
	EventSolicitationResponded EventType = "solicitation.responded"
	EventSolicitationDismissed EventType = "solicitation.dismissed"
	EventSolicitationExpired   EventType = "solicitation.expired"

	// Port events
	EventPortAcquired EventType = "port.acquired"
	EventPortReleased EventType = "port.released"
	EventPortWaiting  EventType = "port.waiting"
	EventPortTimeout  EventType = "port.timeout"
	EventPortConflict EventType = "port.conflict"

	// General
	EventError EventType = "error"
)

// Event represents an event to be sent to clients.
type Event struct {
	Type EventType   `json:"type"`
	Data interface{} `json:"data"`
}

// EventHub manages SSE connections and broadcasts events.
type EventHub struct {
	clients    map[chan Event]bool
	broadcast  chan Event
	register   chan chan Event
	unregister chan chan Event
	mu         sync.RWMutex
}

// NewEventHub creates a new event hub.
func NewEventHub() *EventHub {
	return &EventHub{
		clients:    make(map[chan Event]bool),
		broadcast:  make(chan Event, 100),
		register:   make(chan chan Event),
		unregister: make(chan chan Event),
	}
}

// Run starts the event hub loop.
func (h *EventHub) Run(ctx context.Context) {
	for {
		select {
		case <-ctx.Done():
			h.mu.Lock()
			for client := range h.clients {
				close(client)
				delete(h.clients, client)
			}
			h.mu.Unlock()
			return

		case client := <-h.register:
			h.mu.Lock()
			h.clients[client] = true
			h.mu.Unlock()

		case client := <-h.unregister:
			h.mu.Lock()
			if _, ok := h.clients[client]; ok {
				close(client)
				delete(h.clients, client)
			}
			h.mu.Unlock()

		case event := <-h.broadcast:
			h.mu.RLock()
			for client := range h.clients {
				select {
				case client <- event:
				default:
					// Client buffer full, skip this event
				}
			}
			h.mu.RUnlock()
		}
	}
}

// Broadcast sends an event to all connected clients.
func (h *EventHub) Broadcast(event Event) {
	select {
	case h.broadcast <- event:
	default:
		// Broadcast buffer full, drop event
	}
}

// Subscribe creates a new client subscription.
func (h *EventHub) Subscribe() chan Event {
	client := make(chan Event, 10)
	h.register <- client
	return client
}

// Unsubscribe removes a client subscription.
func (h *EventHub) Unsubscribe(client chan Event) {
	h.unregister <- client
}

// ClientCount returns the number of connected clients.
func (h *EventHub) ClientCount() int {
	h.mu.RLock()
	defer h.mu.RUnlock()
	return len(h.clients)
}

// handleWebSocket handles GET /ws using Server-Sent Events.
func (h *Hub) handleWebSocket(w http.ResponseWriter, r *http.Request) {
	// Set SSE headers
	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("Cache-Control", "no-cache")
	w.Header().Set("Connection", "keep-alive")
	w.Header().Set("Access-Control-Allow-Origin", "*")

	// Check if flusher is supported
	flusher, ok := w.(http.Flusher)
	if !ok {
		http.Error(w, "SSE not supported", http.StatusInternalServerError)
		return
	}

	// Subscribe to events
	client := h.eventHub.Subscribe()
	defer h.eventHub.Unsubscribe(client)

	// Send initial connection event
	h.sendSSE(w, flusher, Event{
		Type: "connected",
		Data: map[string]interface{}{
			"agents_total":   h.agentManager.Count(),
			"agents_running": h.agentManager.CountRunning(),
		},
	})

	// Listen for events until client disconnects
	ctx := r.Context()
	for {
		select {
		case <-ctx.Done():
			return
		case event, ok := <-client:
			if !ok {
				return
			}
			h.sendSSE(w, flusher, event)
		}
	}
}

// sendSSE sends an event in SSE format.
func (h *Hub) sendSSE(w http.ResponseWriter, flusher http.Flusher, event Event) {
	data, err := json.Marshal(event)
	if err != nil {
		return
	}

	fmt.Fprintf(w, "event: %s\n", event.Type)
	fmt.Fprintf(w, "data: %s\n\n", data)
	flusher.Flush()
}
