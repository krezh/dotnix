package handlers

import (
	"log"
	"net/http"
	"strconv"

	"github.com/krezh/nixos-webgui/system"
	"github.com/krezh/nixos-webgui/templates"
)

// HandleGenerations renders the generations page.
func HandleGenerations(w http.ResponseWriter, r *http.Request) {
	generations, err := system.GetGenerations()
	if err != nil {
		log.Printf("Error getting generations: %v", err)
		http.Error(w, "Failed to get generations", http.StatusInternalServerError)
		return
	}

	component := templates.Generations(generations)
	if err := component.Render(w); err != nil {
		log.Printf("Error rendering generations: %v", err)
		http.Error(w, "Failed to render page", http.StatusInternalServerError)
	}
}

// HandleGenerationRollback rolls back to a specific generation.
func HandleGenerationRollback(w http.ResponseWriter, r *http.Request) {
	if err := r.ParseForm(); err != nil {
		http.Error(w, "Failed to parse form", http.StatusBadRequest)
		return
	}

	genNumStr := r.FormValue("generation")
	sudoPassword := r.FormValue("password")

	genNum, err := strconv.Atoi(genNumStr)
	if err != nil {
		http.Error(w, "Invalid generation number", http.StatusBadRequest)
		return
	}

	if err := system.RollbackToGeneration(genNum, sudoPassword); err != nil {
		log.Printf("Rollback failed: %v", err)
		http.Error(w, "Rollback failed: "+err.Error(), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusOK)
	w.Write([]byte("OK"))
}

// HandleGenerationDelete deletes old generations.
func HandleGenerationDelete(w http.ResponseWriter, r *http.Request) {
	if err := r.ParseForm(); err != nil {
		http.Error(w, "Failed to parse form", http.StatusBadRequest)
		return
	}

	keepLastStr := r.FormValue("keep_last")
	sudoPassword := r.FormValue("password")

	keepLast := 0
	if keepLastStr != "" {
		var err error
		keepLast, err = strconv.Atoi(keepLastStr)
		if err != nil {
			http.Error(w, "Invalid keep_last value", http.StatusBadRequest)
			return
		}
	}

	deleted, err := system.DeleteGenerations(keepLast, sudoPassword)
	if err != nil {
		log.Printf("Delete generations failed: %v", err)
		http.Error(w, "Delete failed: "+err.Error(), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusOK)
	w.Write([]byte("Deleted " + strconv.Itoa(deleted) + " generations"))
}
