package handlers

import (
	"fmt"
	"log"
	"net/http"
	"sync"

	"github.com/krezh/nixos-webgui/system"
	"github.com/krezh/nixos-webgui/templates"
)

var (
	updateMutex     sync.Mutex
	updateLogs      []string
	updateRunning   bool
	updateError     string
	updateListeners []chan string
)

// HandleUpdate renders the update page.
func HandleUpdate(w http.ResponseWriter, r *http.Request) {
	component := templates.Update()
	if err := component.Render(r.Context(), w); err != nil {
		log.Printf("Error rendering update page: %v", err)
		http.Error(w, "Failed to render page", http.StatusInternalServerError)
	}
}

// broadcastUpdate sends update to all SSE listeners.
// NOTE: Caller must hold updateMutex!
func broadcastUpdate(message string) {
	log.Printf("Broadcasting message to %d listeners: %q", len(updateListeners), message)
	for _, listener := range updateListeners {
		select {
		case listener <- message:
			log.Printf("  Message sent to listener")
		default:
			log.Printf("  WARNING: Listener channel full, skipping")
		}
	}
}

// HandleUpdateStart starts the build and diff process (step 1).
func HandleUpdateStart(w http.ResponseWriter, r *http.Request) {
	updateMutex.Lock()
	if updateRunning {
		updateMutex.Unlock()
		http.Error(w, "Update already in progress", http.StatusConflict)
		return
	}

	// Reset state
	updateRunning = true
	updateLogs = []string{"Starting build and diff..."}
	updateError = ""
	updateListeners = nil // Clear old listeners
	log.Printf("Update started, initial logs: %v", updateLogs)
	updateMutex.Unlock()

	// Start build and diff in background
	go func() {
		progressChan := make(chan system.UpdateProgress, 100)

		go func() {
			if err := system.BuildDiff(progressChan); err != nil {
				log.Printf("Build/diff failed: %v", err)
			}
		}()

		// Collect progress updates
		for progress := range progressChan {
			updateMutex.Lock()
			if progress.Error != nil {
				updateError = progress.Error.Error()
				updateLogs = append(updateLogs, "ERROR: "+progress.Error.Error())
			} else if progress.Message != "" {
				// Clean the log line
				cleanedLine := system.CleanLogLine(progress.Message)
				if cleanedLine != "" {
					updateLogs = append(updateLogs, cleanedLine)
					broadcastUpdate(cleanedLine)
				}
			}
			updateMutex.Unlock()
		}

		updateMutex.Lock()
		updateRunning = false
		if updateError == "" {
			updateLogs = append(updateLogs, "", "Build and diff completed - review changes above")
			broadcastUpdate("")
			broadcastUpdate("__COMPLETE__")
		}
		updateMutex.Unlock()
	}()

	// Return initial rendering
	component := templates.UpdateProgressInitial(updateLogs)
	if err := component.Render(r.Context(), w); err != nil {
		log.Printf("Error rendering update progress: %v", err)
		http.Error(w, "Failed to render update progress", http.StatusInternalServerError)
	}
}

// HandleUpdateApply applies the built configuration (step 2).
func HandleUpdateApply(w http.ResponseWriter, r *http.Request) {
	updateMutex.Lock()
	if updateRunning {
		updateMutex.Unlock()
		http.Error(w, "Update already in progress", http.StatusConflict)
		return
	}

	// Get sudo password from form
	if err := r.ParseForm(); err != nil {
		updateMutex.Unlock()
		http.Error(w, "Failed to parse form", http.StatusBadRequest)
		return
	}
	sudoPassword := r.FormValue("password")

	// Append to existing logs (keep existing listeners)
	updateRunning = true
	updateLogs = append(updateLogs, "", "=== Applying configuration with sudo ===", "")
	updateError = ""
	// Broadcast the new log lines
	broadcastUpdate("")
	broadcastUpdate("=== Applying configuration with sudo ===")
	broadcastUpdate("")
	updateMutex.Unlock()

	// Start apply in background
	go func() {
		progressChan := make(chan system.UpdateProgress, 100)

		go func() {
			if err := system.ApplyUpdate(progressChan, sudoPassword); err != nil {
				log.Printf("Apply failed: %v", err)
			}
		}()

		// Collect progress updates
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

		updateMutex.Lock()
		updateRunning = false
		if updateError == "" {
			updateLogs = append(updateLogs, "", "Configuration applied successfully!")
			broadcastUpdate("")
			broadcastUpdate("__APPLY_COMPLETE__")
		}
		updateMutex.Unlock()
	}()

	// Just return success - the SSE connection will handle updates
	w.WriteHeader(http.StatusOK)
	w.Write([]byte("OK"))
}

// HandleUpdateStream streams update logs via SSE.
func HandleUpdateStream(w http.ResponseWriter, r *http.Request) {
	log.Println("SSE client connected - ENTERING HANDLER")

	// Set SSE headers
	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("Cache-Control", "no-cache")
	w.Header().Set("Connection", "keep-alive")

	// Stream updates
	flusher, ok := w.(http.Flusher)
	if !ok {
		log.Println("ERROR: Streaming not supported")
		http.Error(w, "Streaming not supported", http.StatusInternalServerError)
		return
	}
	log.Println("Flusher obtained")

	// Send initial comment to establish connection
	log.Println("Sending initial comment")
	fmt.Fprintf(w, ": connected\n\n")
	flusher.Flush()
	log.Println("Initial comment sent")

	// Create a channel for this client
	messageChan := make(chan string, 100)
	log.Println("Message channel created")

	updateMutex.Lock()
	log.Printf("Acquired lock, current listener count: %d", len(updateListeners))
	updateListeners = append(updateListeners, messageChan)
	log.Printf("Added listener, new count: %d", len(updateListeners))

	// Send existing logs immediately
	log.Printf("Sending %d existing log lines to new SSE client", len(updateLogs))
	for i, line := range updateLogs {
		log.Printf("  Log line %d: %q", i, line)
		if line != "" {
			fmt.Fprintf(w, "data: %s\n\n", line)
		}
	}
	flusher.Flush()
	log.Printf("Flushed initial logs, now waiting for updates...")
	updateMutex.Unlock()

	for {
		select {
		case msg := <-messageChan:
			if msg == "__COMPLETE__" {
				// Build complete - show apply button
				fmt.Fprintf(w, "event: complete\ndata: show_apply\n\n")
				flusher.Flush()
				// Don't return - keep connection open for apply phase
			} else if msg == "__APPLY_COMPLETE__" {
				// Apply complete - show success
				fmt.Fprintf(w, "event: complete\ndata: done\n\n")
				flusher.Flush()
				return
			} else if msg == "__FLAKE_COMPLETE__" {
				// Flake update complete - show rollback button
				fmt.Fprintf(w, "event: complete\ndata: flake_done\n\n")
				flusher.Flush()
				return
			} else {
				fmt.Fprintf(w, "data: %s\n\n", msg)
				flusher.Flush()
			}
		case <-r.Context().Done():
			// Client disconnected
			updateMutex.Lock()
			for i, ch := range updateListeners {
				if ch == messageChan {
					updateListeners = append(updateListeners[:i], updateListeners[i+1:]...)
					break
				}
			}
			updateMutex.Unlock()
			close(messageChan)
			return
		}
	}
}

// HandleFlakeUpdate starts the flake update process.
func HandleFlakeUpdate(w http.ResponseWriter, r *http.Request) {
	updateMutex.Lock()
	if updateRunning {
		updateMutex.Unlock()
		http.Error(w, "Update already in progress", http.StatusConflict)
		return
	}

	// Reset state
	updateRunning = true
	updateLogs = []string{"Starting flake update..."}
	updateError = ""
	updateListeners = nil // Clear old listeners
	updateMutex.Unlock()

	// Start flake update in background
	go func() {
		progressChan := make(chan system.UpdateProgress, 100)

		go func() {
			if err := system.FlakeUpdate(progressChan); err != nil {
				log.Printf("Flake update failed: %v", err)
			}
		}()

		// Collect progress updates
		for progress := range progressChan {
			updateMutex.Lock()
			if progress.Error != nil {
				updateError = progress.Error.Error()
				updateLogs = append(updateLogs, "ERROR: "+progress.Error.Error())
			} else if progress.Message != "" {
				// Clean the log line
				cleanedLine := system.CleanLogLine(progress.Message)
				if cleanedLine != "" {
					updateLogs = append(updateLogs, cleanedLine)
					broadcastUpdate(cleanedLine)
				}
			}
			updateMutex.Unlock()
		}

		updateMutex.Lock()
		updateRunning = false
		if updateError == "" {
			updateLogs = append(updateLogs, "", "Flake update completed successfully!")
			broadcastUpdate("")
			broadcastUpdate("__FLAKE_COMPLETE__")
		}
		updateMutex.Unlock()
	}()

	// Return initial rendering with SSE (no apply button for flake update)
	component := templates.UpdateProgressInitial(updateLogs)
	if err := component.Render(r.Context(), w); err != nil {
		log.Printf("Error rendering update progress: %v", err)
		http.Error(w, "Failed to render update progress", http.StatusInternalServerError)
	}
}

// HandleFlakeRollback restores the flake.lock backup.
func HandleFlakeRollback(w http.ResponseWriter, r *http.Request) {
	if err := system.RollbackFlakeLock(); err != nil {
		log.Printf("Flake rollback failed: %v", err)
		http.Error(w, "Rollback failed: "+err.Error(), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusOK)
	w.Write([]byte("Flake lock rolled back successfully"))
}
