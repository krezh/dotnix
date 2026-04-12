package handlers

import (
	"encoding/json"
	"net/http"

	"github.com/go-chi/chi/v5"
)

type route struct {
	Method string `json:"method"`
	Path   string `json:"path"`
}

// HandleAPIRoutes returns an handler that responds with a JSON listing of all routes registered on the given router.
func HandleAPIRoutes(r chi.Router) http.HandlerFunc {
	return func(w http.ResponseWriter, req *http.Request) {
		var routes []route
		chi.Walk(r, func(method, path string, _ http.Handler, _ ...func(http.Handler) http.Handler) error {
			routes = append(routes, route{Method: method, Path: path})
			return nil
		})
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(routes)
	}
}
