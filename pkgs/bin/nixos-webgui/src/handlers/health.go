package handlers

import (
	"encoding/json"
	"net/http"
)

// HandleHealthcheck responds with a simple JSON status indicating the service is alive.
func HandleHealthcheck(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{"status": "ok"})
}
