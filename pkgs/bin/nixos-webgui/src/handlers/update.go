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
	renderComponent(w, r, templates.Update())
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

	updateRunning = true
	updateLogs = []string{"Starting build and diff..."}
	updateError = ""
	updateListeners = nil
	log.Printf("Update started, initial logs: %v", updateLogs)
	updateMutex.Unlock()

	go func() {
		progressChan := make(chan system.UpdateProgress, 100)
		go func() {
			if err := system.BuildDiff(progressChan); err != nil {
				log.Printf("Build/diff failed: %v", err)
			}
		}()

		collectProgressUpdates(progressChan)

		updateMutex.Lock()
		updateRunning = false
		if updateError == "" {
			updateLogs = append(updateLogs, "", "Build and diff completed - review changes above")
			broadcastUpdate("")
			broadcastUpdate("__COMPLETE__")
		}
		updateMutex.Unlock()
	}()

	renderComponent(w, r, templates.UpdateProgressInitial(updateLogs))
}

// HandleUpdateApply applies the already-built configuration (step 2).
func HandleUpdateApply(w http.ResponseWriter, r *http.Request) {
	// Parse form before acquiring the lock.
	sudoPassword, ok := parseSudoPassword(w, r)
	if !ok {
		return
	}

	updateMutex.Lock()
	if updateRunning {
		updateMutex.Unlock()
		http.Error(w, "Update already in progress", http.StatusConflict)
		return
	}

	updateRunning = true
	updateLogs = append(updateLogs, "", "=== Applying configuration with sudo ===", "")
	updateError = ""
	broadcastUpdate("")
	broadcastUpdate("=== Applying configuration with sudo ===")
	broadcastUpdate("")
	updateMutex.Unlock()

	go func() {
		progressChan := make(chan system.UpdateProgress, 100)
		go func() {
			if err := system.ApplyUpdate(progressChan, sudoPassword); err != nil {
				log.Printf("Apply failed: %v", err)
			}
		}()

		collectProgressUpdates(progressChan)

		updateMutex.Lock()
		updateRunning = false
		if updateError == "" {
			updateLogs = append(updateLogs, "", "Configuration applied successfully!")
			broadcastUpdate("")
			broadcastUpdate("__APPLY_COMPLETE__")
		}
		updateMutex.Unlock()
	}()

	w.WriteHeader(http.StatusOK)
	w.Write([]byte("OK"))
}

// HandleUpdateStream streams update logs via SSE.
func HandleUpdateStream(w http.ResponseWriter, r *http.Request) {
	log.Println("SSE client connected - ENTERING HANDLER")

	setSSEHeaders(w)
	flusher := sendSSEConnected(w)
	if flusher == nil {
		log.Println("ERROR: Streaming not supported")
		return
	}
	log.Println("Flusher obtained and initial comment sent")

	messageChan := make(chan string, 100)
	log.Println("Message channel created")

	updateMutex.Lock()
	log.Printf("Acquired lock, current listener count: %d", len(updateListeners))
	updateListeners = append(updateListeners, messageChan)
	log.Printf("Added listener, new count: %d", len(updateListeners))

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
				fmt.Fprintf(w, "event: complete\ndata: show_apply\n\n")
				flusher.Flush()
			} else if msg == "__APPLY_COMPLETE__" {
				fmt.Fprintf(w, "event: complete\ndata: done\n\n")
				flusher.Flush()
				return
			} else if msg == "__FLAKE_COMPLETE__" {
				fmt.Fprintf(w, "event: complete\ndata: flake_done\n\n")
				flusher.Flush()
				return
			} else {
				fmt.Fprintf(w, "data: %s\n\n", msg)
				flusher.Flush()
			}
		case <-r.Context().Done():
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

	updateRunning = true
	updateLogs = []string{"Starting flake update..."}
	updateError = ""
	updateListeners = nil
	updateMutex.Unlock()

	go func() {
		progressChan := make(chan system.UpdateProgress, 100)
		go func() {
			if err := system.FlakeUpdate(progressChan); err != nil {
				log.Printf("Flake update failed: %v", err)
			}
		}()

		collectProgressUpdates(progressChan)

		updateMutex.Lock()
		updateRunning = false
		if updateError == "" {
			updateLogs = append(updateLogs, "", "Flake update completed successfully!")
			broadcastUpdate("")
			broadcastUpdate("__FLAKE_COMPLETE__")
		}
		updateMutex.Unlock()
	}()

	renderComponent(w, r, templates.UpdateProgressInitial(updateLogs))
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
