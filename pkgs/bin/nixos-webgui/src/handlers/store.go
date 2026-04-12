package handlers

import (
	"encoding/json"
	"log"
	"net/http"
	"sync"
	"time"

	"github.com/krezh/nixos-webgui/system"
	"github.com/krezh/nixos-webgui/templates"
)

var (
	storeStatsMutex  sync.RWMutex
	cachedStoreStats *system.StoreStats
	storeStatsTime   time.Time
)

// HandleStore renders the store page.
func HandleStore(w http.ResponseWriter, r *http.Request) {
	// Get cached store stats if available
	storeStatsMutex.RLock()
	cachedStats := cachedStoreStats
	cachedTime := storeStatsTime
	storeStatsMutex.RUnlock()

	component := templates.Store(cachedStats, cachedTime)
	if err := component.Render(w); err != nil {
		log.Printf("Error rendering store: %v", err)
		http.Error(w, "Failed to render page", http.StatusInternalServerError)
	}
}

// HandleStoreStats returns store statistics as JSON.
// Returns cached data if available and not forcing refresh.
func HandleStoreStats(w http.ResponseWriter, r *http.Request) {
	forceRefresh := r.URL.Query().Get("refresh") == "true"

	storeStatsMutex.RLock()
	hasCached := cachedStoreStats != nil
	storeStatsMutex.RUnlock()

	// If not forcing refresh and we have cached data, return it
	if !forceRefresh && hasCached {
		storeStatsMutex.RLock()
		stats := cachedStoreStats
		statsTime := storeStatsTime
		storeStatsMutex.RUnlock()

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]interface{}{
			"total_size": stats.TotalSize,
			"dead_size":  stats.DeadSize,
			"alive_size": stats.AliveSize,
			"cached_at":  statsTime.Format(time.RFC3339),
			"cached":     true,
		})
		return
	}

	// Fetch fresh stats
	stats, err := system.GetStoreStats()
	if err != nil {
		log.Printf("Error getting store stats: %v", err)
		http.Error(w, "Failed to get store stats", http.StatusInternalServerError)
		return
	}

	// Cache the results
	storeStatsMutex.Lock()
	cachedStoreStats = stats
	storeStatsTime = time.Now()
	storeStatsMutex.Unlock()

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"total_size": stats.TotalSize,
		"dead_size":  stats.DeadSize,
		"alive_size": stats.AliveSize,
		"cached_at":  storeStatsTime.Format(time.RFC3339),
		"cached":     false,
	})
}

// HandleGarbageCollection runs garbage collection.
func HandleGarbageCollection(w http.ResponseWriter, r *http.Request) {
	if err := r.ParseForm(); err != nil {
		http.Error(w, "Failed to parse form", http.StatusBadRequest)
		return
	}

	sudoPassword := r.FormValue("password")

	progressChan := make(chan system.UpdateProgress, 100)

	go func() {
		if err := system.RunGarbageCollection(sudoPassword, progressChan); err != nil {
			log.Printf("GC failed: %v", err)
		}
	}()

	// Wait for completion
	var lastMessage string
	for progress := range progressChan {
		if progress.Message != "" {
			lastMessage = progress.Message
		}
	}

	// Invalidate cache after GC
	storeStatsMutex.Lock()
	cachedStoreStats = nil
	storeStatsMutex.Unlock()

	w.WriteHeader(http.StatusOK)
	w.Write([]byte("Garbage collection completed: " + lastMessage))
}

// HandleStoreOptimize runs store optimization.
func HandleStoreOptimize(w http.ResponseWriter, r *http.Request) {
	if err := r.ParseForm(); err != nil {
		http.Error(w, "Failed to parse form", http.StatusBadRequest)
		return
	}

	sudoPassword := r.FormValue("password")

	progressChan := make(chan system.UpdateProgress, 100)

	go func() {
		if err := system.OptimizeStore(sudoPassword, progressChan); err != nil {
			log.Printf("Optimize failed: %v", err)
		}
	}()

	// Wait for completion
	var lastMessage string
	for progress := range progressChan {
		if progress.Message != "" {
			lastMessage = progress.Message
		}
	}

	// Invalidate cache after optimization
	storeStatsMutex.Lock()
	cachedStoreStats = nil
	storeStatsMutex.Unlock()

	w.WriteHeader(http.StatusOK)
	w.Write([]byte("Store optimization completed: " + lastMessage))
}
