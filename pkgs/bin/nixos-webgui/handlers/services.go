package handlers

import (
	"bufio"
	"fmt"
	"log"
	"net/http"
	"strings"

	"github.com/go-chi/chi/v5"
	"github.com/krezh/nixos-webgui/system"
	"github.com/krezh/nixos-webgui/templates"
)

// HandleServices renders the services page.
func HandleServices(w http.ResponseWriter, r *http.Request) {
	filter := r.URL.Query().Get("filter")
	if filter == "" {
		filter = "all"
	}

	services, err := system.GetServices(filter)
	if err != nil {
		log.Printf("Error getting services: %v", err)
		http.Error(w, "Failed to get services", http.StatusInternalServerError)
		return
	}

	component := templates.Services(services, filter)
	if err := component.Render(r.Context(), w); err != nil {
		log.Printf("Error rendering services: %v", err)
		http.Error(w, "Failed to render page", http.StatusInternalServerError)
	}
}

// HandleServiceStart starts a service.
func HandleServiceStart(w http.ResponseWriter, r *http.Request) {
	serviceName := chi.URLParam(r, "name")
	if !strings.HasSuffix(serviceName, ".service") {
		serviceName += ".service"
	}

	if err := system.StartService(serviceName); err != nil {
		log.Printf("Error starting service %s: %v", serviceName, err)
		http.Error(w, "Failed to start service", http.StatusInternalServerError)
		return
	}

	// Trigger page reload
	w.Header().Set("HX-Refresh", "true")
	w.WriteHeader(http.StatusOK)
}

// HandleServiceStop stops a service.
func HandleServiceStop(w http.ResponseWriter, r *http.Request) {
	serviceName := chi.URLParam(r, "name")
	if !strings.HasSuffix(serviceName, ".service") {
		serviceName += ".service"
	}

	if err := system.StopService(serviceName); err != nil {
		log.Printf("Error stopping service %s: %v", serviceName, err)
		http.Error(w, "Failed to stop service", http.StatusInternalServerError)
		return
	}

	// Trigger page reload
	w.Header().Set("HX-Refresh", "true")
	w.WriteHeader(http.StatusOK)
}

// HandleServiceRestart restarts a service.
func HandleServiceRestart(w http.ResponseWriter, r *http.Request) {
	serviceName := chi.URLParam(r, "name")
	if !strings.HasSuffix(serviceName, ".service") {
		serviceName += ".service"
	}

	if err := system.RestartService(serviceName); err != nil {
		log.Printf("Error restarting service %s: %v", serviceName, err)
		http.Error(w, "Failed to restart service", http.StatusInternalServerError)
		return
	}

	// Trigger page reload
	w.Header().Set("HX-Refresh", "true")
	w.WriteHeader(http.StatusOK)
}

// HandleServiceLogs returns logs for a service.
func HandleServiceLogs(w http.ResponseWriter, r *http.Request) {
	serviceName := chi.URLParam(r, "name")
	if !strings.HasSuffix(serviceName, ".service") {
		serviceName += ".service"
	}

	logs, err := system.GetServiceLogs(serviceName, 100)
	if err != nil {
		log.Printf("Error getting logs for service %s: %v", serviceName, err)
		http.Error(w, "Failed to get service logs", http.StatusInternalServerError)
		return
	}

	component := templates.ServiceLogs(serviceName, logs)
	if err := component.Render(r.Context(), w); err != nil {
		log.Printf("Error rendering logs: %v", err)
		http.Error(w, "Failed to render logs", http.StatusInternalServerError)
	}
}

// HandleServiceLogsStream streams service logs in real-time via SSE.
func HandleServiceLogsStream(w http.ResponseWriter, r *http.Request) {
	serviceName := chi.URLParam(r, "name")
	if !strings.HasSuffix(serviceName, ".service") {
		serviceName += ".service"
	}

	// Set SSE headers
	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("Cache-Control", "no-cache")
	w.Header().Set("Connection", "keep-alive")

	// Send initial connection confirmation
	fmt.Fprintf(w, ": connected\n\n")
	if f, ok := w.(http.Flusher); ok {
		f.Flush()
	}

	// Start streaming logs
	cmd, err := system.StreamServiceLogs(serviceName, 50)
	if err != nil {
		log.Printf("Error starting log stream for service %s: %v", serviceName, err)
		fmt.Fprintf(w, "event: error\ndata: Failed to start log stream\n\n")
		return
	}

	stdout, err := cmd.StdoutPipe()
	if err != nil {
		log.Printf("Error creating stdout pipe for service %s: %v", serviceName, err)
		fmt.Fprintf(w, "event: error\ndata: Failed to create log stream\n\n")
		return
	}

	if err := cmd.Start(); err != nil {
		log.Printf("Error starting journalctl for service %s: %v", serviceName, err)
		fmt.Fprintf(w, "event: error\ndata: Failed to start journalctl\n\n")
		return
	}

	// Create a context that gets cancelled when the client disconnects
	ctx := r.Context()

	// Cleanup on exit
	defer func() {
		if cmd.Process != nil {
			cmd.Process.Kill()
		}
		cmd.Wait()
	}()

	// Stream logs line by line
	scanner := bufio.NewScanner(stdout)
	for scanner.Scan() {
		select {
		case <-ctx.Done():
			// Client disconnected
			return
		default:
			line := scanner.Text()
			// Send each log line as an SSE event
			fmt.Fprintf(w, "data: %s\n\n", line)
			if f, ok := w.(http.Flusher); ok {
				f.Flush()
			}
		}
	}

	if err := scanner.Err(); err != nil {
		log.Printf("Error reading logs for service %s: %v", serviceName, err)
	}
}
