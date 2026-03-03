package handlers

import (
	"bytes"
	"fmt"
	"log"
	"net/http"
	"sync"
	"time"

	"github.com/krezh/nixos-webgui/system"
	"github.com/krezh/nixos-webgui/templates"
)

var (
	dashboardStatsMutex  sync.RWMutex
	cachedDashboardStats *dashboardStats
	dashboardStatsTime   time.Time
	statsRefreshStarted  bool
	statsRefreshMutex    sync.Mutex
	cpuHistory           []float64 // Last 15 minutes of CPU usage (450 samples at 2s intervals)
	cpuHistoryMutex      sync.RWMutex
)

const maxCPUHistorySamples = 450 // 15 minutes at 2-second intervals

type dashboardStats struct {
	Info         *system.SystemInfo
	CPU          *system.CPUStats
	CPUHistory   []float64
	Mem          *system.MemoryStats
	Disks        []system.DiskStats
	Load         []float64
	GPUs         []system.GPUStats
	Network      []system.NetworkStats
	Temperatures *system.TemperatureStats
	TopProcesses []system.ProcessInfo
}

// StartStatsRefreshWorker starts a background goroutine that refreshes stats every 2 seconds.
// This ensures the server controls fetching, not the clients.
func StartStatsRefreshWorker() {
	statsRefreshMutex.Lock()
	if statsRefreshStarted {
		statsRefreshMutex.Unlock()
		return
	}
	statsRefreshStarted = true
	statsRefreshMutex.Unlock()

	go func() {
		// Initial refresh
		refreshDashboardCache()

		ticker := time.NewTicker(2 * time.Second)
		defer ticker.Stop()

		for range ticker.C {
			refreshDashboardCache()
		}
	}()

	log.Println("Dashboard stats refresh worker started (2s interval)")
}

// refreshDashboardCache fetches fresh stats and caches them.
func refreshDashboardCache() error {
	info, err := system.GetSystemInfo()
	if err != nil {
		return err
	}

	cpu, err := system.GetCPUStats()
	if err != nil {
		return err
	}

	mem, err := system.GetMemoryStats()
	if err != nil {
		return err
	}

	disks, err := system.GetDiskStats()
	if err != nil {
		return err
	}

	load, err := system.GetLoadAverage()
	if err != nil {
		log.Printf("Error getting load average: %v", err)
		load = []float64{0, 0, 0}
	}

	gpus, err := system.GetGPUStats()
	if err != nil {
		log.Printf("Error getting GPU stats: %v", err)
	}

	network, err := system.GetNetworkStats()
	if err != nil {
		log.Printf("Error getting network stats: %v", err)
	}

	temperatures, err := system.GetTemperatureStats()
	if err != nil {
		log.Printf("Error getting temperature stats: %v", err)
	}

	topProcs, err := system.GetTopProcesses(10)
	if err != nil {
		log.Printf("Error getting top processes: %v", err)
	}

	// Update CPU history
	cpuHistoryMutex.Lock()
	cpuHistory = append(cpuHistory, cpu.UsagePercent)
	// Keep only last 15 minutes (450 samples at 2s intervals)
	if len(cpuHistory) > maxCPUHistorySamples {
		cpuHistory = cpuHistory[len(cpuHistory)-maxCPUHistorySamples:]
	}
	historyCopy := make([]float64, len(cpuHistory))
	copy(historyCopy, cpuHistory)
	cpuHistoryMutex.Unlock()

	dashboardStatsMutex.Lock()
	cachedDashboardStats = &dashboardStats{
		Info:         info,
		CPU:          cpu,
		CPUHistory:   historyCopy,
		Mem:          mem,
		Disks:        disks,
		Load:         load,
		GPUs:         gpus,
		Network:      network,
		Temperatures: temperatures,
		TopProcesses: topProcs,
	}
	dashboardStatsTime = time.Now()
	dashboardStatsMutex.Unlock()

	return nil
}

// HandleDashboard renders the dashboard page with cached stats.
func HandleDashboard(w http.ResponseWriter, r *http.Request) {
	// Start the background refresh worker if not already started
	StartStatsRefreshWorker()

	// Use cached stats (always fresh because of background worker)
	dashboardStatsMutex.RLock()
	stats := cachedDashboardStats
	dashboardStatsMutex.RUnlock()

	// If no cache yet, wait for first refresh
	if stats == nil {
		// Trigger immediate refresh
		refreshDashboardCache()

		dashboardStatsMutex.RLock()
		stats = cachedDashboardStats
		dashboardStatsMutex.RUnlock()

		if stats == nil {
			http.Error(w, "Failed to get system information", http.StatusInternalServerError)
			return
		}
	}

	component := templates.Dashboard(stats.Info, stats.CPU, stats.CPUHistory, stats.Mem, stats.Disks, stats.Load, stats.GPUs, stats.Network, stats.Temperatures, stats.TopProcesses)
	if err := component.Render(r.Context(), w); err != nil {
		log.Printf("Error rendering dashboard: %v", err)
		http.Error(w, "Failed to render page", http.StatusInternalServerError)
	}
}

// HandleStatsStream streams stats updates via SSE (server controls polling).
func HandleStatsStream(w http.ResponseWriter, r *http.Request) {
	// Set SSE headers
	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("Cache-Control", "no-cache")
	w.Header().Set("Connection", "keep-alive")

	flusher, ok := w.(http.Flusher)
	if !ok {
		log.Println("ERROR: Streaming not supported")
		http.Error(w, "Streaming not supported", http.StatusInternalServerError)
		return
	}

	// Send initial ping
	fmt.Fprintf(w, ": connected\n\n")
	flusher.Flush()

	// Create ticker for 2-second intervals (server controls the rate)
	ticker := time.NewTicker(2 * time.Second)
	defer ticker.Stop()

	// Send stats from cache (background worker keeps it fresh)
	sendStats := func() error {
		// Get cached stats (refreshed by background worker)
		dashboardStatsMutex.RLock()
		stats := cachedDashboardStats
		dashboardStatsMutex.RUnlock()

		if stats == nil {
			return fmt.Errorf("no stats available")
		}

		// Render template to buffer
		var buf bytes.Buffer
		component := templates.StatsGrid(stats.CPU, stats.CPUHistory, stats.Mem, stats.Disks, stats.Load, stats.GPUs, stats.Network, stats.Temperatures, stats.TopProcesses)
		if err := component.Render(r.Context(), &buf); err != nil {
			log.Printf("Error rendering stats: %v", err)
			return err
		}

		// Send as SSE event
		fmt.Fprintf(w, "data: %s\n\n", buf.String())
		flusher.Flush()
		return nil
	}

	// Send initial stats
	if err := sendStats(); err != nil {
		return
	}

	// Stream stats every 3 seconds
	for {
		select {
		case <-ticker.C:
			if err := sendStats(); err != nil {
				return
			}
		case <-r.Context().Done():
			// Client disconnected
			log.Println("Dashboard stats stream client disconnected")
			return
		}
	}
}
