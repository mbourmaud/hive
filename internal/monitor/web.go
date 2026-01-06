package monitor

import (
	_ "embed"
	"encoding/json"
	"fmt"
	"net/http"
	"sync"

	"github.com/gorilla/websocket"
)

//go:embed web/index.html
var indexHTML string

var upgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool {
		return true
	},
}

type WebServer struct {
	hubURL    string
	client    *HubClient
	clients   map[*websocket.Conn]bool
	clientsMu sync.RWMutex
	broadcast chan []byte
}

func NewWebServer(hubURL string) *WebServer {
	return &WebServer{
		hubURL:    hubURL,
		client:    NewHubClient(hubURL),
		clients:   make(map[*websocket.Conn]bool),
		broadcast: make(chan []byte, 256),
	}
}

func (s *WebServer) handleIndex(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/html")
	w.Write([]byte(indexHTML))
}

func (s *WebServer) handleAPI(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")

	agents, _ := s.client.GetAgents()
	tasks, _ := s.client.GetTasks()
	solicitations, _ := s.client.GetSolicitations()

	data := map[string]interface{}{
		"agents":        agents,
		"tasks":         tasks,
		"solicitations": solicitations,
	}

	json.NewEncoder(w).Encode(data)
}

func (s *WebServer) handleWS(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		return
	}
	defer conn.Close()

	s.clientsMu.Lock()
	s.clients[conn] = true
	s.clientsMu.Unlock()

	defer func() {
		s.clientsMu.Lock()
		delete(s.clients, conn)
		s.clientsMu.Unlock()
	}()

	for {
		_, _, err := conn.ReadMessage()
		if err != nil {
			break
		}
	}
}

func (s *WebServer) broadcastLoop() {
	s.client.OnEvent = func(event Event) {
		data, err := json.Marshal(event)
		if err != nil {
			return
		}
		s.broadcast <- data
	}

	_ = s.client.ConnectWebSocket()

	for msg := range s.broadcast {
		s.clientsMu.RLock()
		for client := range s.clients {
			err := client.WriteMessage(websocket.TextMessage, msg)
			if err != nil {
				client.Close()
			}
		}
		s.clientsMu.RUnlock()
	}
}

func (s *WebServer) Start(port int) error {
	go s.broadcastLoop()

	mux := http.NewServeMux()
	mux.HandleFunc("/", s.handleIndex)
	mux.HandleFunc("/api/data", s.handleAPI)
	mux.HandleFunc("/ws", s.handleWS)

	addr := fmt.Sprintf(":%d", port)
	return http.ListenAndServe(addr, mux)
}

func RunWeb(port int, hubURL string) error {
	server := NewWebServer(hubURL)
	return server.Start(port)
}
