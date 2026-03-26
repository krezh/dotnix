package handlers

import (
	"bytes"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"sync"
	"time"

	"maragu.dev/gomponents/html"

	"github.com/krezh/nixos-webgui/system"
	"github.com/krezh/nixos-webgui/templates"
)

// renderBufPool reuses render buffers across SSE ticks to reduce GC pressure.
var renderBufPool = sync.Pool{
	New: func() interface{} { return new(bytes.Buffer) },
}

// netIfacePoint is a single network interface data point sent via SSE.
type netIfacePoint struct {
	Idx    int    `json:"i"`
	RxRate uint64 `json:"rx"`
	TxRate uint64 `json:"tx"`
}

var (
	dashboardStatsMutex  sync.RWMutex
	cachedDashboardStats *dashboardStats
	dashboardStatsTime   time.Time
	statsRefreshStarted  bool
	statsRefreshMutex    sync.Mutex
	cpuHistoryMutex      sync.Mutex
)

const maxCPUHistorySamples = 450 // 15 minutes at 2-second intervals

// cpuHistoryRing is a fixed-size ring buffer for CPU history samples.
var (
	cpuHistoryRing  [maxCPUHistorySamples]float64
	cpuHistoryHead  int // index of oldest sample
	cpuHistoryCount int // number of valid samples (0..maxCPUHistorySamples)
)

// cpuHistorySnapshot returns a []float64 slice in chronological order.
// Must be called with cpuHistoryMutex held.
func cpuHistorySnapshot() []float64 {
	if cpuHistoryCount == 0 {
		return nil
	}
	out := make([]float64, cpuHistoryCount)
	if cpuHistoryCount < maxCPUHistorySamples {
		copy(out, cpuHistoryRing[:cpuHistoryCount])
	} else {
		// Ring is full: oldest is at head, wraps around.
		n := copy(out, cpuHistoryRing[cpuHistoryHead:])
		copy(out[n:], cpuHistoryRing[:cpuHistoryHead])
	}
	return out
}

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
	// Pre-rendered SSE frame bytes — built once per tick, broadcast to all clients.
	sseHTML     []byte // "data: <html>\n\n"
	sseNetdata  []byte // "event: netdata\ndata: <json>\n\n"
	sseCPU      []byte // "event: cpudata\ndata: <float>\n\n"
	sseGPU      []byte // "event: gpudata\ndata: <json>\n\n"
	sseTempdata []byte // "event: tempdata\ndata: <json>\n\n"
	sseMemdata  []byte // "event: memdata\ndata: <json>\n\n"
}

// StartStatsRefreshWorker starts a background goroutine that refreshes stats every 2 seconds.
func StartStatsRefreshWorker() {
	statsRefreshMutex.Lock()
	if statsRefreshStarted {
		statsRefreshMutex.Unlock()
		return
	}
	statsRefreshStarted = true
	statsRefreshMutex.Unlock()

	go func() {
		refreshDashboardCache()

		ticker := time.NewTicker(2 * time.Second)
		defer ticker.Stop()

		for range ticker.C {
			refreshDashboardCache()
		}
	}()

	log.Println("Dashboard stats refresh worker started (2s interval)")
}

// refreshDashboardCache fetches all stats concurrently, then caches the result
// along with pre-rendered SSE frame bytes.
func refreshDashboardCache() error {
	type result struct {
		info    *system.SystemInfo
		cpu     *system.CPUStats
		mem     *system.MemoryStats
		disks   []system.DiskStats
		load    []float64
		gpus    []system.GPUStats
		network []system.NetworkStats
		temps   *system.TemperatureStats
		procs   []system.ProcessInfo
	}

	var (
		res  result
		mu   sync.Mutex
		wg   sync.WaitGroup
		errs []string
	)

	addErr := func(msg string) {
		mu.Lock()
		errs = append(errs, msg)
		mu.Unlock()
	}

	wg.Add(9)

	go func() {
		defer wg.Done()
		v, err := system.GetSystemInfo()
		if err != nil {
			addErr(fmt.Sprintf("system info: %v", err))
			return
		}
		mu.Lock()
		res.info = v
		mu.Unlock()
	}()

	go func() {
		defer wg.Done()
		v, err := system.GetCPUStats()
		if err != nil {
			addErr(fmt.Sprintf("cpu stats: %v", err))
			return
		}
		mu.Lock()
		res.cpu = v
		mu.Unlock()
	}()

	go func() {
		defer wg.Done()
		v, err := system.GetMemoryStats()
		if err != nil {
			addErr(fmt.Sprintf("mem stats: %v", err))
			return
		}
		mu.Lock()
		res.mem = v
		mu.Unlock()
	}()

	go func() {
		defer wg.Done()
		v, err := system.GetDiskStats()
		if err != nil {
			addErr(fmt.Sprintf("disk stats: %v", err))
			return
		}
		mu.Lock()
		res.disks = v
		mu.Unlock()
	}()

	go func() {
		defer wg.Done()
		v, err := system.GetLoadAverage()
		if err != nil {
			log.Printf("Error getting load average: %v", err)
			v = []float64{0, 0, 0}
		}
		mu.Lock()
		res.load = v
		mu.Unlock()
	}()

	go func() {
		defer wg.Done()
		v, err := system.GetGPUStats()
		if err != nil {
			log.Printf("Error getting GPU stats: %v", err)
		}
		mu.Lock()
		res.gpus = v
		mu.Unlock()
	}()

	go func() {
		defer wg.Done()
		v, err := system.GetNetworkStats()
		if err != nil {
			log.Printf("Error getting network stats: %v", err)
		}
		mu.Lock()
		res.network = v
		mu.Unlock()
	}()

	go func() {
		defer wg.Done()
		v, err := system.GetTemperatureStats()
		if err != nil {
			log.Printf("Error getting temperature stats: %v", err)
		}
		mu.Lock()
		res.temps = v
		mu.Unlock()
	}()

	go func() {
		defer wg.Done()
		v, err := system.GetTopProcesses(10)
		if err != nil {
			log.Printf("Error getting top processes: %v", err)
		}
		mu.Lock()
		res.procs = v
		mu.Unlock()
	}()

	wg.Wait()

	if len(errs) > 0 {
		return fmt.Errorf("refresh errors: %v", errs)
	}

	// Update CPU history ring buffer.
	cpuHistoryMutex.Lock()
	if res.cpu != nil {
		if cpuHistoryCount < maxCPUHistorySamples {
			cpuHistoryRing[cpuHistoryCount] = res.cpu.UsagePercent
			cpuHistoryCount++
		} else {
			cpuHistoryRing[cpuHistoryHead] = res.cpu.UsagePercent
			cpuHistoryHead = (cpuHistoryHead + 1) % maxCPUHistorySamples
		}
	}
	historyCopy := cpuHistorySnapshot()
	cpuHistoryMutex.Unlock()

	stats := &dashboardStats{
		Info:         res.info,
		CPU:          res.cpu,
		CPUHistory:   historyCopy,
		Mem:          res.mem,
		Disks:        res.disks,
		Load:         res.load,
		GPUs:         res.gpus,
		Network:      res.network,
		Temperatures: res.temps,
		TopProcesses: res.procs,
	}

	// Pre-render SSE frames once so all connected clients can share the bytes.
	if res.cpu != nil {
		stats.sseHTML = buildSSEHTML(stats)
		stats.sseNetdata = buildSSENetdata(res.network)
		stats.sseCPU = []byte(fmt.Sprintf("event: cpudata\ndata: %.4f\n\n", res.cpu.UsagePercent))
		if len(res.gpus) > 0 {
			stats.sseGPU = buildSSEGPUdata(res.gpus)
		}
		if res.temps != nil {
			stats.sseTempdata = buildSSETempdata(res.temps, res.gpus)
		}
		if res.mem != nil {
			stats.sseMemdata = buildSSEMemdata(res.mem)
		}
	}

	dashboardStatsMutex.Lock()
	cachedDashboardStats = stats
	dashboardStatsTime = time.Now()
	dashboardStatsMutex.Unlock()

	return nil
}

// buildSSEHTML renders the StatsGrid template to a pre-built SSE "data:" frame.
// Uses a pooled buffer to avoid per-tick allocations.
func buildSSEHTML(stats *dashboardStats) []byte {
	buf := renderBufPool.Get().(*bytes.Buffer)
	buf.Reset()
	defer renderBufPool.Put(buf)

	component := html.Div(
		templates.StatsGrid(
			stats.CPU, stats.CPUHistory, stats.Mem, stats.Disks,
			stats.Load, stats.GPUs, stats.Network, stats.Temperatures, stats.TopProcesses,
		),
	)
	if err := component.Render(buf); err != nil {
		log.Printf("Error pre-rendering SSE HTML: %v", err)
		return nil
	}
	frame := make([]byte, 0, len("data: ")+buf.Len()+2)
	frame = append(frame, "data: "...)
	frame = append(frame, buf.Bytes()...)
	frame = append(frame, '\n', '\n')
	return frame
}

// buildSSENetdata builds the pre-serialised "event: netdata" SSE frame.
func buildSSENetdata(network []system.NetworkStats) []byte {
	points := make([]netIfacePoint, len(network))
	for i, iface := range network {
		points[i] = netIfacePoint{Idx: i, RxRate: iface.RxRate, TxRate: iface.TxRate}
	}
	jsonBytes, err := json.Marshal(points)
	if err != nil {
		return nil
	}
	frame := make([]byte, 0, len("event: netdata\ndata: ")+len(jsonBytes)+2)
	frame = append(frame, "event: netdata\ndata: "...)
	frame = append(frame, jsonBytes...)
	frame = append(frame, '\n', '\n')
	return frame
}

// gpuDataPoint is a single GPU data point sent via SSE.
type gpuDataPoint struct {
	Idx  int     `json:"i"`
	Util float64 `json:"util"`
	Mem  float64 `json:"mem"`
}

// buildSSEGPUdata builds the pre-serialised "event: gpudata" SSE frame.
func buildSSEGPUdata(gpus []system.GPUStats) []byte {
	points := make([]gpuDataPoint, len(gpus))
	for i, gpu := range gpus {
		points[i] = gpuDataPoint{Idx: i, Util: gpu.UtilizationGPU, Mem: gpu.MemoryPercent}
	}
	jsonBytes, err := json.Marshal(points)
	if err != nil {
		return nil
	}
	frame := make([]byte, 0, len("event: gpudata\ndata: ")+len(jsonBytes)+2)
	frame = append(frame, "event: gpudata\ndata: "...)
	frame = append(frame, jsonBytes...)
	frame = append(frame, '\n', '\n')
	return frame
}

// tempDataPoint carries temperature values for a single SSE tempdata frame.
type tempDataPoint struct {
	CPUTemp  float64        `json:"cpu"`
	GPUTemps []gpuTempPoint `json:"gpus,omitempty"`
}

// gpuTempPoint carries the index and temperature for a single GPU.
type gpuTempPoint struct {
	Idx  int     `json:"i"`
	Temp float64 `json:"temp"`
}

// buildSSETempdata builds the pre-serialised "event: tempdata" SSE frame.
func buildSSETempdata(temps *system.TemperatureStats, gpus []system.GPUStats) []byte {
	point := tempDataPoint{CPUTemp: temps.CPUTemp}
	for i, gpu := range gpus {
		if gpu.Temperature > 0 {
			point.GPUTemps = append(point.GPUTemps, gpuTempPoint{Idx: i, Temp: gpu.Temperature})
		}
	}
	jsonBytes, err := json.Marshal(point)
	if err != nil {
		return nil
	}
	frame := make([]byte, 0, len("event: tempdata\ndata: ")+len(jsonBytes)+2)
	frame = append(frame, "event: tempdata\ndata: "...)
	frame = append(frame, jsonBytes...)
	frame = append(frame, '\n', '\n')
	return frame
}

// memDataPoint carries RAM and swap percent for a single SSE memdata frame.
type memDataPoint struct {
	RAM  float64 `json:"ram"`
	Swap float64 `json:"swap"`
}

// buildSSEMemdata builds the pre-serialised "event: memdata" SSE frame.
func buildSSEMemdata(mem *system.MemoryStats) []byte {
	point := memDataPoint{RAM: mem.UsedPercent, Swap: mem.SwapPercent}
	jsonBytes, err := json.Marshal(point)
	if err != nil {
		return nil
	}
	frame := make([]byte, 0, len("event: memdata\ndata: ")+len(jsonBytes)+2)
	frame = append(frame, "event: memdata\ndata: "...)
	frame = append(frame, jsonBytes...)
	frame = append(frame, '\n', '\n')
	return frame
}

// backgroundCtx is a long-lived context used for background template rendering.
// We use context.Background() equivalent — templates only use it for cancellation checks.
var backgroundCtx = &fakeCtx{}

// fakeCtx is a minimal context.Context that never cancels, used for background renders.
type fakeCtx struct{}

func (*fakeCtx) Deadline() (time.Time, bool)       { return time.Time{}, false }
func (*fakeCtx) Done() <-chan struct{}             { return nil }
func (*fakeCtx) Err() error                        { return nil }
func (*fakeCtx) Value(key interface{}) interface{} { return nil }

// HandleDashboard renders the dashboard page with cached stats.
func HandleDashboard(w http.ResponseWriter, r *http.Request) {
	StartStatsRefreshWorker()

	dashboardStatsMutex.RLock()
	stats := cachedDashboardStats
	dashboardStatsMutex.RUnlock()

	if stats == nil {
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
	if err := component.Render(w); err != nil {
		log.Printf("Error rendering dashboard: %v", err)
		http.Error(w, "Failed to render page", http.StatusInternalServerError)
	}
}

// HandleStatsStream streams stats updates via SSE (server controls polling).
// Each client receives the pre-rendered SSE frame bytes built by the background worker.
func HandleStatsStream(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("Cache-Control", "no-cache")
	w.Header().Set("Connection", "keep-alive")

	flusher, ok := w.(http.Flusher)
	if !ok {
		log.Println("ERROR: Streaming not supported")
		http.Error(w, "Streaming not supported", http.StatusInternalServerError)
		return
	}

	fmt.Fprintf(w, ": connected\n\n")
	flusher.Flush()

	ticker := time.NewTicker(2 * time.Second)
	defer ticker.Stop()

	sendStats := func() bool {
		dashboardStatsMutex.RLock()
		stats := cachedDashboardStats
		dashboardStatsMutex.RUnlock()

		if stats == nil || stats.sseHTML == nil {
			return true // skip this tick, cache not ready
		}

		w.Write(stats.sseHTML)
		if stats.sseNetdata != nil {
			w.Write(stats.sseNetdata)
		}
		if stats.sseCPU != nil {
			w.Write(stats.sseCPU)
		}
		if stats.sseGPU != nil {
			w.Write(stats.sseGPU)
		}
		if stats.sseTempdata != nil {
			w.Write(stats.sseTempdata)
		}
		if stats.sseMemdata != nil {
			w.Write(stats.sseMemdata)
		}
		flusher.Flush()
		return true
	}

	if !sendStats() {
		return
	}

	for {
		select {
		case <-ticker.C:
			if !sendStats() {
				return
			}
		case <-r.Context().Done():
			log.Println("Dashboard stats stream client disconnected")
			return
		}
	}
}
