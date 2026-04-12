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

// normalizeServiceName ensures the service name ends with ".service".
func normalizeServiceName(name string) string {
	if !strings.HasSuffix(name, ".service") {
		return name + ".service"
	}
	return name
}

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

	renderComponent(w, r, templates.Services(services, filter))
}

// HandleServiceStart starts a service.
func HandleServiceStart(w http.ResponseWriter, r *http.Request) {
	serviceName := normalizeServiceName(chi.URLParam(r, "name"))

	sudoPassword, ok := parseSudoPassword(w, r)
	if !ok {
		return
	}

	if err := system.StartServiceWithSudo(serviceName, sudoPassword); err != nil {
		log.Printf("Error starting service %s: %v", serviceName, err)
		http.Error(w, "Failed to start service", http.StatusInternalServerError)
		return
	}

	w.Header().Set("HX-Refresh", "true")
	w.WriteHeader(http.StatusOK)
}

// HandleServiceStop stops a service.
func HandleServiceStop(w http.ResponseWriter, r *http.Request) {
	serviceName := normalizeServiceName(chi.URLParam(r, "name"))

	sudoPassword, ok := parseSudoPassword(w, r)
	if !ok {
		return
	}

	if err := system.StopServiceWithSudo(serviceName, sudoPassword); err != nil {
		log.Printf("Error stopping service %s: %v", serviceName, err)
		http.Error(w, "Failed to stop service", http.StatusInternalServerError)
		return
	}

	w.Header().Set("HX-Refresh", "true")
	w.WriteHeader(http.StatusOK)
}

// HandleServiceRestart restarts a service.
func HandleServiceRestart(w http.ResponseWriter, r *http.Request) {
	serviceName := normalizeServiceName(chi.URLParam(r, "name"))

	sudoPassword, ok := parseSudoPassword(w, r)
	if !ok {
		return
	}

	if err := system.RestartServiceWithSudo(serviceName, sudoPassword); err != nil {
		log.Printf("Error restarting service %s: %v", serviceName, err)
		http.Error(w, "Failed to restart service", http.StatusInternalServerError)
		return
	}

	w.Header().Set("HX-Refresh", "true")
	w.WriteHeader(http.StatusOK)
}

// HandleServiceLogs returns logs for a service.
func HandleServiceLogs(w http.ResponseWriter, r *http.Request) {
	serviceName := normalizeServiceName(chi.URLParam(r, "name"))

	logs, err := system.GetServiceLogs(serviceName, 100)
	if err != nil {
		log.Printf("Error getting logs for service %s: %v", serviceName, err)
		http.Error(w, "Failed to get service logs", http.StatusInternalServerError)
		return
	}

	renderComponent(w, r, templates.ServiceLogs(serviceName, logs))
}

// HandleServiceLogsStream streams service logs in real-time via SSE.
func HandleServiceLogsStream(w http.ResponseWriter, r *http.Request) {
	serviceName := normalizeServiceName(chi.URLParam(r, "name"))

	setSSEHeaders(w)
	flusher := sendSSEConnected(w)
	if flusher == nil {
		return
	}

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

	ctx := r.Context()

	defer func() {
		if cmd.Process != nil {
			cmd.Process.Kill()
		}
		cmd.Wait()
	}()

	scanner := bufio.NewScanner(stdout)
	for scanner.Scan() {
		select {
		case <-ctx.Done():
			return
		default:
			fmt.Fprintf(w, "data: %s\n\n", scanner.Text())
			flusher.Flush()
		}
	}

	if err := scanner.Err(); err != nil {
		log.Printf("Error reading logs for service %s: %v", serviceName, err)
	}
}
