package handlers

import (
	"fmt"
	"log"
	"net/http"

	g "maragu.dev/gomponents"
	"github.com/krezh/nixos-webgui/system"
)

// setSSEHeaders sets the required headers for a Server-Sent Events response.
func setSSEHeaders(w http.ResponseWriter) {
	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("Cache-Control", "no-cache")
	w.Header().Set("Connection", "keep-alive")
}

// sendSSEConnected sends the initial SSE connection confirmation comment.
// Returns the flusher if streaming is supported, or writes an error and returns nil.
func sendSSEConnected(w http.ResponseWriter) http.Flusher {
	flusher, ok := w.(http.Flusher)
	if !ok {
		http.Error(w, "Streaming not supported", http.StatusInternalServerError)
		return nil
	}
	fmt.Fprintf(w, ": connected\n\n")
	flusher.Flush()
	return flusher
}

// renderComponent executes the gomponents component and writes the result to the HTTP response.
//
// Sends a 500 error if rendering fails.
func renderComponent(w http.ResponseWriter, r *http.Request, component g.Node) {
	if err := component.Render(w); err != nil {
		log.Printf("Error rendering component: %v", err)
		http.Error(w, "Failed to render page", http.StatusInternalServerError)
	}
}

// parseSudoPassword parses and returns the sudo password from a form POST.
// Returns an error and writes a 400 response if the form cannot be parsed.
func parseSudoPassword(w http.ResponseWriter, r *http.Request) (string, bool) {
	if err := r.ParseForm(); err != nil {
		http.Error(w, "Failed to parse form", http.StatusBadRequest)
		return "", false
	}
	return r.FormValue("password"), true
}

// collectProgressUpdates reads from progressChan, appends cleaned log lines to
// updateLogs, and broadcasts them to all SSE listeners. Must be called with
// updateMutex unlocked; acquires the lock per-message.
func collectProgressUpdates(progressChan <-chan system.UpdateProgress) {
	for progress := range progressChan {
		updateMutex.Lock()
		if progress.Error != nil {
			updateError = progress.Error.Error()
			updateLogs = append(updateLogs, "ERROR: "+progress.Error.Error())
			broadcastUpdate("ERROR: " + progress.Error.Error())
		} else if progress.Message != "" {
			cleanedLine := system.CleanLogLine(progress.Message)
			if cleanedLine != "" {
				updateLogs = append(updateLogs, cleanedLine)
				broadcastUpdate(cleanedLine)
			}
		}
		updateMutex.Unlock()
	}
}
