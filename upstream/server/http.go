package server

import (
	"net/http"

	"github.com/gorilla/websocket"
)

var upgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool { return true },
}

// NewHTTPHandler returns a new ServeMux configured with HTTP and WebSocket routes.
func NewHTTPHandler() http.Handler {
	mux := http.NewServeMux()

	// Root handler
	mux.HandleFunc("/", func(w http.ResponseWriter, _ *http.Request) {
		_, _ = w.Write([]byte("hello http\n"))
	})

	// User API handler
	mux.HandleFunc("/api/users/", func(w http.ResponseWriter, r *http.Request) {
		id := r.URL.Path[len("/api/users/"):]
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"id":"` + id + `"}`))
	})

	// WebSocket echo handler
	mux.HandleFunc("/ws", func(w http.ResponseWriter, r *http.Request) {
		c, err := upgrader.Upgrade(w, r, nil)
		if err != nil {
			return
		}
		defer c.Close()

		for {
			mt, msg, err := c.ReadMessage()
			if err != nil {
				return
			}
			_ = c.WriteMessage(mt, append([]byte("echo: "), msg...))
		}
	})

	return mux
}
