package main

import (
	"embed"
	"io/fs"
	"log"
	"net/http"

	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"
	"github.com/krezh/nixos-webgui/handlers"
)

//go:embed static
var staticFiles embed.FS

func main() {
	r := chi.NewRouter()

	// Middleware
	r.Use(middleware.Logger)
	r.Use(middleware.Recoverer)
	r.Use(middleware.Compress(5))

	// Static files
	staticFS, err := fs.Sub(staticFiles, "static")
	if err != nil {
		log.Fatal(err)
	}
	r.Handle("/static/*", http.StripPrefix("/static/", http.FileServer(http.FS(staticFS))))

	// Routes
	r.Get("/", handlers.HandleDashboard)
	r.Get("/services", handlers.HandleServices)
	r.Get("/update", handlers.HandleUpdate)
	r.Get("/generations", handlers.HandleGenerations)
	r.Get("/store", handlers.HandleStore)

	// API Routes
	r.Route("/api", func(r chi.Router) {
		// Stats streaming via SSE (server controls polling)
		r.Get("/stats/stream", handlers.HandleStatsStream)

		// Service management
		r.Post("/services/{name}/start", handlers.HandleServiceStart)
		r.Post("/services/{name}/stop", handlers.HandleServiceStop)
		r.Post("/services/{name}/restart", handlers.HandleServiceRestart)
		r.Get("/services/{name}/logs", handlers.HandleServiceLogs)
		r.Get("/services/{name}/logs/stream", handlers.HandleServiceLogsStream)

		// Update endpoints
		r.Get("/update/git-status", handlers.HandleGitStatus)
		r.Get("/update/git-status-dot", handlers.HandleGitStatusDot)
		r.Post("/update/git-status/refresh", handlers.HandleGitStatusRefresh)
		r.Post("/update/start", handlers.HandleUpdateStart)
		r.Post("/update/apply", handlers.HandleUpdateApply)
		r.Post("/update/flake", handlers.HandleFlakeUpdate)
		r.Post("/update/flake/rollback", handlers.HandleFlakeRollback)
		r.Get("/update/stream", handlers.HandleUpdateStream)

		// Generation endpoints
		r.Post("/generations/rollback", handlers.HandleGenerationRollback)
		r.Post("/generations/delete", handlers.HandleGenerationDelete)

		// Store endpoints
		r.Get("/store/stats", handlers.HandleStoreStats)
		r.Post("/store/gc", handlers.HandleGarbageCollection)
		r.Post("/store/optimize", handlers.HandleStoreOptimize)
	})

	log.Println("Starting NixOS WebGUI on http://localhost:8080")
	if err := http.ListenAndServe("0.0.0.0:8080", r); err != nil {
		log.Fatal(err)
	}
}
