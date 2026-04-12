package system

import (
	"bufio"
	"fmt"
	"os"
	"os/exec"
	"sort"
	"strconv"
	"strings"
	"sync"
	"syscall"

	"github.com/shirou/gopsutil/v4/net"
)

// CPUStats represents CPU usage statistics.
type CPUStats struct {
	UsagePercent  float64
	PhysicalCores int
	LogicalCores  int
	PerCoreUsage  []float64
	ModelName     string
	CPUCount      int // Number of physical CPU sockets
}

// GPUStats represents GPU usage statistics.
type GPUStats struct {
	Name              string
	MemoryUsed        uint64
	MemoryTotal       uint64
	MemoryPercent     float64
	Temperature       float64
	UtilizationGPU    float64
	UtilizationMemory float64
	UtilHistory       []float64 // last 90 UtilizationGPU samples
	MemHistory        []float64 // last 90 MemoryPercent samples
	TempHistory       []float64 // last 90 Temperature samples
}

// MemoryStats represents memory usage statistics.
type MemoryStats struct {
	Total       uint64
	Used        uint64
	Available   uint64
	UsedPercent float64
	SwapTotal   uint64
	SwapUsed    uint64
	SwapPercent float64
	RAMHistory  []float64 // last 90 UsedPercent samples
	SwapHistory []float64 // last 90 SwapPercent samples
}

// DiskStats represents disk usage for a mount point.
type DiskStats struct {
	MountPoint  string
	Total       uint64
	Used        uint64
	Available   uint64
	UsedPercent float64
}

// SystemInfo represents general system information.
type SystemInfo struct {
	Hostname      string
	Uptime        uint64
	OS            string
	Platform      string
	KernelVersion string
	NixOSVersion  string
}

// NetworkInterfaceType categorizes a network interface.
type NetworkInterfaceType string

const (
	NetTypeEthernet  NetworkInterfaceType = "ethernet"
	NetTypeWifi      NetworkInterfaceType = "wifi"
	NetTypeDocker    NetworkInterfaceType = "docker"
	NetTypeBridge    NetworkInterfaceType = "bridge"
	NetTypeVPN       NetworkInterfaceType = "vpn"
	NetTypeWireguard NetworkInterfaceType = "wireguard"
	NetTypeVLAN      NetworkInterfaceType = "vlan"
	NetTypeOther     NetworkInterfaceType = "other"
)

// NetworkStats represents network interface statistics.
type NetworkStats struct {
	Interface    string
	Type         NetworkInterfaceType
	Status       string
	HardwareAddr string   // MAC address
	IPAddresses  []string // all addresses (IPv4 + IPv6)
	Gateway      string   // default gateway for this interface (IPv4)
	DNSServers   []string // DNS servers from /etc/resolv.conf
	IPSource     string   // "dhcp", "static", or ""
	VLANID       int      // 802.1q VLAN ID (0 if not a VLAN interface)
	VLANParent   string   // parent interface name for VLAN interfaces
	BytesSent    uint64
	BytesRecv    uint64
	PacketsSent  uint64
	PacketsRecv  uint64
	TxRate       uint64   // bytes/sec since last sample
	RxRate       uint64   // bytes/sec since last sample
	RxHistory    []uint64 // last 30 RxRate samples
	TxHistory    []uint64 // last 30 TxRate samples
}

// TemperatureStats represents categorized temperature sensor readings.
type TemperatureStats struct {
	CPUTemp        float64
	GPUTemp        float64
	MemoryTemp     float64
	CPUTempHistory []float64 // last 90 CPUTemp samples
}

// ProcessInfo represents a running process.
type ProcessInfo struct {
	PID         int32
	Name        string
	CPUPercent  float64
	MemPercent  float32
	MemoryBytes uint64
}

// ---------------------------------------------------------------------------
// Package-level caches for values that never change at runtime.
// ---------------------------------------------------------------------------

var (
	// cpuInfoCache is populated once on the first GetCPUStats call.
	cpuInfoCache     *cpuInfoStatic
	cpuInfoCacheOnce sync.Once

	// memTotalBytes is read once from /proc/meminfo (MemTotal never changes).
	memTotalBytes     uint64
	memTotalBytesOnce sync.Once

	// pageSize is a syscall constant; cache it to avoid repeated Getpagesize calls.
	cachedPageSize     uint64
	cachedPageSizeOnce sync.Once

	// staticSysInfo holds system fields that never change at runtime.
	staticSysInfo     *staticSysInfoFields
	staticSysInfoOnce sync.Once
)

// staticSysInfoFields holds the system information fields that are constant.
type staticSysInfoFields struct {
	hostname      string
	kernelVersion string
	nixosVersion  string
}

// cpuInfoStatic holds the parts of CPU information that do not change.
type cpuInfoStatic struct {
	logicalCores  int
	physicalCores int
	modelName     string
	cpuCount      int // number of physical sockets
}

// loadCPUInfoOnce parses /proc/cpuinfo once and fills cpuInfoCache.
func loadCPUInfoOnce() {
	cpuInfoCacheOnce.Do(func() {
		f, err := os.Open("/proc/cpuinfo")
		if err != nil {
			cpuInfoCache = &cpuInfoStatic{logicalCores: 1, physicalCores: 1, modelName: "Unknown", cpuCount: 1}
			return
		}
		defer f.Close()

		var (
			logicalCores  int
			modelName     string
			physicalIDs   = map[string]bool{}
			coreIDsByPhys = map[string]map[string]bool{} // physicalID -> set of core IDs
		)

		scanner := bufio.NewScanner(f)
		var currentPhysID, currentCoreID string
		for scanner.Scan() {
			line := scanner.Text()
			if strings.HasPrefix(line, "processor") {
				logicalCores++
			} else if strings.HasPrefix(line, "model name") {
				if modelName == "" {
					if idx := strings.Index(line, ":"); idx >= 0 {
						modelName = strings.TrimSpace(line[idx+1:])
					}
				}
			} else if strings.HasPrefix(line, "physical id") {
				if idx := strings.Index(line, ":"); idx >= 0 {
					currentPhysID = strings.TrimSpace(line[idx+1:])
					physicalIDs[currentPhysID] = true
				}
			} else if strings.HasPrefix(line, "core id") {
				if idx := strings.Index(line, ":"); idx >= 0 {
					currentCoreID = strings.TrimSpace(line[idx+1:])
					if coreIDsByPhys[currentPhysID] == nil {
						coreIDsByPhys[currentPhysID] = map[string]bool{}
					}
					coreIDsByPhys[currentPhysID][currentCoreID] = true
				}
			}
		}

		if logicalCores == 0 {
			logicalCores = 1
		}
		if modelName == "" {
			modelName = "Unknown"
		}

		// Count unique physical cores across all sockets.
		physicalCores := 0
		for _, cores := range coreIDsByPhys {
			physicalCores += len(cores)
		}
		// Fallback: if /proc/cpuinfo lacks core id (e.g. VM), use logical count.
		if physicalCores == 0 {
			physicalCores = logicalCores
		}

		cpuCount := len(physicalIDs)
		if cpuCount == 0 {
			cpuCount = 1
		}

		cpuInfoCache = &cpuInfoStatic{
			logicalCores:  logicalCores,
			physicalCores: physicalCores,
			modelName:     modelName,
			cpuCount:      cpuCount,
		}
	})
}

// getPageSize returns the system page size, reading it only once.
func getPageSize() uint64 {
	cachedPageSizeOnce.Do(func() {
		cachedPageSize = uint64(os.Getpagesize())
	})
	return cachedPageSize
}

// getMemTotalBytes returns total physical memory in bytes, reading /proc/meminfo only once.
func getMemTotalBytes() uint64 {
	memTotalBytesOnce.Do(func() {
		f, err := os.Open("/proc/meminfo")
		if err != nil {
			return
		}
		defer f.Close()
		scanner := bufio.NewScanner(f)
		for scanner.Scan() {
			line := scanner.Text()
			if strings.HasPrefix(line, "MemTotal:") {
				fields := strings.Fields(line)
				if len(fields) >= 2 {
					if kb, err := strconv.ParseUint(fields[1], 10, 64); err == nil {
						memTotalBytes = kb * 1024
					}
				}
				break
			}
		}
	})
	return memTotalBytes
}

// ---------------------------------------------------------------------------
// CPU stats — /proc/stat read once; static info from cache.
// ---------------------------------------------------------------------------

// procStatReadBuf is sized to hold the cpu lines of /proc/stat.
// 33 lines × ~60 bytes each = ~2 KiB; 8 KiB is comfortable for 256 cores.
var procStatReadBuf = make([]byte, 8192)
var procStatReadMu sync.Mutex

// readProcStat reads /proc/stat once and returns per-core usage percentages
// and the aggregate total tick count, using zero-allocation byte-level parsing.
func readProcStat() (perCorePct []float64, totalTicks uint64, err error) {
	procStatReadMu.Lock()
	defer procStatReadMu.Unlock()

	f, err := os.Open("/proc/stat")
	if err != nil {
		return nil, 0, err
	}
	n, readErr := f.Read(procStatReadBuf)
	f.Close()
	if n == 0 {
		return nil, 0, fmt.Errorf("empty /proc/stat")
	}
	if readErr != nil && n == 0 {
		return nil, 0, readErr
	}
	data := procStatReadBuf[:n]

	// Parse all lines starting with "cpu".
	// Each line: "cpu[N] u n s i w q h g sg\n"
	// We need: total = sum of all fields; idle = fields[3]+fields[4]
	type coreLine struct {
		isAggregate bool
		coreIdx     int // -1 for aggregate
		total       uint64
		idle        uint64
	}
	// Pre-allocate: we'll collect aggregate + per-core lines.
	lines := make([]coreLine, 0, 64)

	pos := 0
	for pos < n {
		// Check if line starts with "cpu".
		if pos+3 > n || data[pos] != 'c' || data[pos+1] != 'p' || data[pos+2] != 'u' {
			break
		}
		// Find end of line.
		end := pos
		for end < n && data[end] != '\n' {
			end++
		}
		line := data[pos:end]

		// Skip "cpu" prefix, then optional digits for core index, then space.
		p := 3
		isAggregate := p >= len(line) || line[p] == ' '
		// Advance past core number digits.
		for p < len(line) && line[p] >= '0' && line[p] <= '9' {
			p++
		}
		// Skip space.
		if p < len(line) && line[p] == ' ' {
			p++
		}

		// Parse up to 10 tick fields.
		var ticks [10]uint64
		tickIdx := 0
		for tickIdx < 10 && p < len(line) {
			// Skip spaces.
			for p < len(line) && line[p] == ' ' {
				p++
			}
			if p >= len(line) {
				break
			}
			var v uint64
			for p < len(line) && line[p] >= '0' && line[p] <= '9' {
				v = v*10 + uint64(line[p]-'0')
				p++
			}
			ticks[tickIdx] = v
			tickIdx++
		}

		var total uint64
		for _, t := range ticks {
			total += t
		}
		idle := ticks[3] + ticks[4]

		lines = append(lines, coreLine{
			isAggregate: isAggregate,
			total:       total,
			idle:        idle,
		})

		if end < n {
			end++ // skip '\n'
		}
		pos = end
	}

	// Compute per-core percentages from delta vs last snapshot.
	procStatMu.Lock()
	defer procStatMu.Unlock()

	// Make sure prevCoreTicks slice is large enough.
	// Use slice-based prev storage indexed by core number.
	numCores := len(lines) - 1 // exclude aggregate
	if numCores < 0 {
		numCores = 0
	}
	if cap(prevCoreTicksSlice) < len(lines) {
		prevCoreTicksSlice = make([]cpuCoreTicks, len(lines))
	}
	if len(prevCoreTicksSlice) < len(lines) {
		prevCoreTicksSlice = prevCoreTicksSlice[:len(lines)]
	}

	perCorePct = make([]float64, 0, numCores)
	for i, cur := range lines {
		if cur.isAggregate {
			totalTicks = cur.total
			prevTotalTicks = cur.total
			prevCoreTicksSlice[i] = cpuCoreTicks{total: cur.total, idle: cur.idle}
			continue
		}
		prev := prevCoreTicksSlice[i]
		deltaTotal := cur.total - prev.total
		deltaIdle := cur.idle - prev.idle
		if deltaTotal == 0 {
			perCorePct = append(perCorePct, 0)
		} else {
			perCorePct = append(perCorePct, (1.0-float64(deltaIdle)/float64(deltaTotal))*100.0)
		}
		prevCoreTicksSlice[i] = cpuCoreTicks{total: cur.total, idle: cur.idle}
	}

	return perCorePct, totalTicks, nil
}

type cpuCoreTicks struct {
	name  string
	total uint64
	idle  uint64
}

var (
	procStatMu         sync.Mutex
	prevCoreTicksSlice []cpuCoreTicks // indexed same as lines[] in readProcStat
)

// GetCPUStats retrieves current CPU usage statistics.
// Reads /proc/stat once; static CPU info is cached after first call.
func GetCPUStats() (*CPUStats, error) {
	loadCPUInfoOnce()

	perCore, _, err := readProcStat()
	if err != nil {
		return nil, fmt.Errorf("failed to read /proc/stat: %w", err)
	}

	// Calculate overall from per-core averages.
	var total float64
	for _, usage := range perCore {
		total += usage
	}
	overall := 0.0
	if len(perCore) > 0 {
		overall = total / float64(len(perCore))
	}

	return &CPUStats{
		UsagePercent:  overall,
		PhysicalCores: cpuInfoCache.physicalCores,
		LogicalCores:  cpuInfoCache.logicalCores,
		PerCoreUsage:  perCore,
		ModelName:     cpuInfoCache.modelName,
		CPUCount:      cpuInfoCache.cpuCount,
	}, nil
}

// ---------------------------------------------------------------------------
// Memory stats — /proc/meminfo read once per call (values change each tick).
// ---------------------------------------------------------------------------

// GetMemoryStats retrieves current memory usage statistics.
// Parses /proc/meminfo once per call instead of calling two gopsutil functions.
func GetMemoryStats() (*MemoryStats, error) {
	f, err := os.Open("/proc/meminfo")
	if err != nil {
		return nil, fmt.Errorf("failed to open /proc/meminfo: %w", err)
	}
	defer f.Close()

	var (
		memTotal     uint64
		memFree      uint64
		memAvailable uint64
		swapTotal    uint64
		swapFree     uint64
		found        int
	)

	scanner := bufio.NewScanner(f)
	for scanner.Scan() && found < 5 {
		line := scanner.Text()
		fields := strings.Fields(line)
		if len(fields) < 2 {
			continue
		}
		val, err := strconv.ParseUint(fields[1], 10, 64)
		if err != nil {
			continue
		}
		switch fields[0] {
		case "MemTotal:":
			memTotal = val * 1024
			found++
		case "MemFree:":
			memFree = val * 1024
			found++
		case "MemAvailable:":
			memAvailable = val * 1024
			found++
		case "SwapTotal:":
			swapTotal = val * 1024
			found++
		case "SwapFree:":
			swapFree = val * 1024
			found++
		}
	}
	if err := scanner.Err(); err != nil {
		return nil, fmt.Errorf("error reading /proc/meminfo: %w", err)
	}

	memUsed := memTotal - memFree - (memAvailable - memFree) // approximate: total - available
	// Simpler and more standard: used = total - available
	memUsed = memTotal - memAvailable
	usedPercent := 0.0
	if memTotal > 0 {
		usedPercent = float64(memUsed) / float64(memTotal) * 100.0
	}

	swapUsed := swapTotal - swapFree
	swapPercent := 0.0
	if swapTotal > 0 {
		swapPercent = float64(swapUsed) / float64(swapTotal) * 100.0
	}

	memHistoryMu.Lock()
	ramHistoryData = appendGPUHistory(ramHistoryData, usedPercent)
	swapHistoryData = appendGPUHistory(swapHistoryData, swapPercent)
	ramSnap := append([]float64(nil), ramHistoryData...)
	swapSnap := append([]float64(nil), swapHistoryData...)
	memHistoryMu.Unlock()

	return &MemoryStats{
		Total:       memTotal,
		Used:        memUsed,
		Available:   memAvailable,
		UsedPercent: usedPercent,
		SwapTotal:   swapTotal,
		SwapUsed:    swapUsed,
		SwapPercent: swapPercent,
		RAMHistory:  ramSnap,
		SwapHistory: swapSnap,
	}, nil
}

// ---------------------------------------------------------------------------
// Disk stats — constant filter maps lifted to package level.
// ---------------------------------------------------------------------------

var (
	skipFsTypes = map[string]bool{
		"proc":        true,
		"sysfs":       true,
		"devtmpfs":    true,
		"devpts":      true,
		"tmpfs":       true,
		"cgroup":      true,
		"cgroup2":     true,
		"pstore":      true,
		"bpf":         true,
		"tracefs":     true,
		"debugfs":     true,
		"securityfs":  true,
		"hugetlbfs":   true,
		"mqueue":      true,
		"autofs":      true,
		"fusectl":     true,
		"fuse.portal": true,
		"nsfs":        true,
		"ramfs":       true,
	}

	skipMountPoints = map[string]bool{
		"/proc":     true,
		"/sys":      true,
		"/dev":      true,
		"/run":      true,
		"/tmp":      true,
		"/var/tmp":  true,
		"/dev/shm":  true,
		"/run/user": true,
	}
)

// GetDiskStats retrieves disk usage for all mounted filesystems.
func GetDiskStats() ([]DiskStats, error) {
	var stats []DiskStats

	file, err := os.Open("/proc/mounts")
	if err != nil {
		return nil, fmt.Errorf("failed to open /proc/mounts: %w", err)
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	seen := make(map[string]bool)

	for scanner.Scan() {
		fields := strings.Fields(scanner.Text())
		if len(fields) < 3 {
			continue
		}

		device := fields[0]
		mountPoint := fields[1]
		fsType := fields[2]

		if shouldSkipFilesystem(device, mountPoint, fsType) {
			continue
		}

		if seen[mountPoint] {
			continue
		}
		seen[mountPoint] = true

		var stat syscall.Statfs_t
		if err := syscall.Statfs(mountPoint, &stat); err != nil {
			continue
		}

		total := stat.Blocks * uint64(stat.Bsize)
		if total == 0 {
			continue
		}

		available := stat.Bavail * uint64(stat.Bsize)
		used := total - (stat.Bfree * uint64(stat.Bsize))
		usedPercent := float64(used) / float64(total) * 100

		stats = append(stats, DiskStats{
			MountPoint:  mountPoint,
			Total:       total,
			Used:        used,
			Available:   available,
			UsedPercent: usedPercent,
		})
	}

	if err := scanner.Err(); err != nil {
		return stats, fmt.Errorf("error reading /proc/mounts: %w", err)
	}

	return stats, nil
}

// shouldSkipFilesystem determines if a filesystem should be skipped.
func shouldSkipFilesystem(device, mountPoint, fsType string) bool {
	if skipFsTypes[fsType] {
		return true
	}

	if !strings.HasPrefix(device, "/dev/") &&
		!strings.HasPrefix(device, "//") &&
		!strings.Contains(device, ":") {
		return true
	}

	if skipMountPoints[mountPoint] {
		return true
	}

	if strings.HasPrefix(mountPoint, "/run/user/") {
		return true
	}

	return false
}

// ---------------------------------------------------------------------------
// System info.
// ---------------------------------------------------------------------------

// loadStaticSysInfoOnce populates staticSysInfo once.
func loadStaticSysInfoOnce() {
	staticSysInfoOnce.Do(func() {
		hostname, _ := os.Hostname()
		kernelVersion, _ := readKernelVersion()
		nixosVersion, _ := getNixOSVersion()
		if hostname == "" {
			hostname = "unknown"
		}
		if kernelVersion == "" {
			kernelVersion = "unknown"
		}
		if nixosVersion == "" {
			nixosVersion = "unknown"
		}
		staticSysInfo = &staticSysInfoFields{
			hostname:      hostname,
			kernelVersion: kernelVersion,
			nixosVersion:  nixosVersion,
		}
	})
}

// GetSystemInfo retrieves general system information.
// Only uptime is re-read each call; hostname/kernel/NixOS version are cached.
func GetSystemInfo() (*SystemInfo, error) {
	loadStaticSysInfoOnce()

	uptime, err := readUptimeSeconds()
	if err != nil {
		uptime = 0
	}

	return &SystemInfo{
		Hostname:      staticSysInfo.hostname,
		Uptime:        uptime,
		OS:            "linux",
		Platform:      "nixos",
		KernelVersion: staticSysInfo.kernelVersion,
		NixOSVersion:  staticSysInfo.nixosVersion,
	}, nil
}

// readUptimeSeconds reads system uptime in seconds from /proc/uptime.
func readUptimeSeconds() (uint64, error) {
	f, err := os.Open("/proc/uptime")
	if err != nil {
		return 0, err
	}
	defer f.Close()
	scanner := bufio.NewScanner(f)
	if !scanner.Scan() {
		return 0, fmt.Errorf("empty /proc/uptime")
	}
	fields := strings.Fields(scanner.Text())
	if len(fields) < 1 {
		return 0, fmt.Errorf("unexpected /proc/uptime format")
	}
	v, err := strconv.ParseFloat(fields[0], 64)
	if err != nil {
		return 0, err
	}
	return uint64(v), nil
}

// readKernelVersion reads the kernel version from /proc/version.
func readKernelVersion() (string, error) {
	data, err := os.ReadFile("/proc/version")
	if err != nil {
		return "", err
	}
	// Format: "Linux version 6.x.y ..."
	fields := strings.Fields(string(data))
	if len(fields) >= 3 {
		return fields[2], nil
	}
	return strings.TrimSpace(string(data)), nil
}

// getNixOSVersion retrieves the NixOS build ID from /etc/os-release.
func getNixOSVersion() (string, error) {
	file, err := os.Open("/etc/os-release")
	if err != nil {
		return "", err
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := scanner.Text()
		if strings.HasPrefix(line, "BUILD_ID=") {
			parts := strings.SplitN(line, "=", 2)
			if len(parts) == 2 {
				return strings.Trim(parts[1], "\""), nil
			}
		}
	}

	if err := scanner.Err(); err != nil {
		return "", err
	}

	return "unknown", nil
}

// ---------------------------------------------------------------------------
// GPU stats.
// ---------------------------------------------------------------------------

const maxGPUHistorySamples = 90

// gpuHistoryCache stores per-GPU utilisation, VRAM, and temperature history across ticks.
var (
	gpuHistoryMu     sync.Mutex
	gpuUtilHistories [][]float64
	gpuMemHistories  [][]float64
	gpuTempHistories [][]float64
)

// cpuTempHistoryCache stores rolling CPU temperature history across ticks.
var (
	cpuTempHistoryMu   sync.Mutex
	cpuTempHistoryData []float64
)

// memHistoryCache stores rolling RAM and swap percent history across ticks.
var (
	memHistoryMu    sync.Mutex
	ramHistoryData  []float64
	swapHistoryData []float64
)

// appendGPUHistory appends a new value to a rolling history slice (max 90 entries).
func appendGPUHistory(history []float64, value float64) []float64 {
	history = append(history, value)
	if len(history) > maxGPUHistorySamples {
		history = history[len(history)-maxGPUHistorySamples:]
	}
	return history
}

// GetGPUStats retrieves GPU statistics.
func GetGPUStats() ([]GPUStats, error) {
	var gpus []GPUStats
	if g := getNVIDIAGPUs(); len(g) > 0 {
		gpus = g
	} else if g := getAMDGPUs(); len(g) > 0 {
		gpus = g
	}
	if len(gpus) == 0 {
		return nil, nil
	}

	gpuHistoryMu.Lock()
	// Grow slices if the number of GPUs increased.
	for len(gpuUtilHistories) < len(gpus) {
		gpuUtilHistories = append(gpuUtilHistories, nil)
		gpuMemHistories = append(gpuMemHistories, nil)
		gpuTempHistories = append(gpuTempHistories, nil)
	}
	for i := range gpus {
		gpuUtilHistories[i] = appendGPUHistory(gpuUtilHistories[i], gpus[i].UtilizationGPU)
		gpuMemHistories[i] = appendGPUHistory(gpuMemHistories[i], gpus[i].MemoryPercent)
		gpuTempHistories[i] = appendGPUHistory(gpuTempHistories[i], gpus[i].Temperature)
		gpus[i].UtilHistory = append([]float64(nil), gpuUtilHistories[i]...)
		gpus[i].MemHistory = append([]float64(nil), gpuMemHistories[i]...)
		gpus[i].TempHistory = append([]float64(nil), gpuTempHistories[i]...)
	}
	gpuHistoryMu.Unlock()

	return gpus, nil
}

func getNVIDIAGPUs() []GPUStats {
	cmd := exec.Command("nvidia-smi", "--query-gpu=name,memory.used,memory.total,temperature.gpu,utilization.gpu,utilization.memory", "--format=csv,noheader,nounits")
	output, err := cmd.Output()
	if err != nil {
		return nil
	}

	var gpus []GPUStats
	scanner := bufio.NewScanner(strings.NewReader(string(output)))
	for scanner.Scan() {
		line := scanner.Text()
		parts := strings.Split(line, ", ")
		if len(parts) < 6 {
			continue
		}

		memUsed, _ := strconv.ParseUint(strings.TrimSpace(parts[1]), 10, 64)
		memTotal, _ := strconv.ParseUint(strings.TrimSpace(parts[2]), 10, 64)
		temp, _ := strconv.ParseFloat(strings.TrimSpace(parts[3]), 64)
		utilGPU, _ := strconv.ParseFloat(strings.TrimSpace(parts[4]), 64)
		utilMem, _ := strconv.ParseFloat(strings.TrimSpace(parts[5]), 64)

		memUsedBytes := memUsed * 1024 * 1024
		memTotalBytes := memTotal * 1024 * 1024
		memPercent := 0.0
		if memTotalBytes > 0 {
			memPercent = float64(memUsedBytes) / float64(memTotalBytes) * 100
		}

		gpus = append(gpus, GPUStats{
			Name:              strings.TrimSpace(parts[0]),
			MemoryUsed:        memUsedBytes,
			MemoryTotal:       memTotalBytes,
			MemoryPercent:     memPercent,
			Temperature:       temp,
			UtilizationGPU:    utilGPU,
			UtilizationMemory: utilMem,
		})
	}

	return gpus
}

// getVulkanGPUNames retrieves GPU names from vulkaninfo.
func getVulkanGPUNames() map[string]string {
	namesByDeviceID := make(map[string]string)

	cmd := exec.Command("vulkaninfo")
	output, err := cmd.Output()
	if err != nil {
		return namesByDeviceID
	}

	scanner := bufio.NewScanner(strings.NewReader(string(output)))
	var currentDeviceID string
	for scanner.Scan() {
		line := scanner.Text()
		// Parse deviceID (hex format: 0x7550)
		if strings.Contains(line, "deviceID") && strings.Contains(line, "=") {
			parts := strings.SplitN(line, "=", 2)
			if len(parts) == 2 {
				deviceID := strings.TrimSpace(parts[1])
				// Remove 0x prefix and convert to uppercase for matching
				deviceID = strings.TrimPrefix(deviceID, "0x")
				currentDeviceID = strings.ToUpper(deviceID)
			}
		}
		// Parse deviceName and associate with current deviceID
		if strings.Contains(line, "deviceName") && strings.Contains(line, "=") && currentDeviceID != "" {
			parts := strings.SplitN(line, "=", 2)
			if len(parts) == 2 {
				name := strings.TrimSpace(parts[1])
				if idx := strings.Index(name, "("); idx != -1 {
					name = strings.TrimSpace(name[:idx])
				}
				if !strings.Contains(name, "llvmpipe") {
					namesByDeviceID[currentDeviceID] = name
				}
			}
		}
	}

	return namesByDeviceID
}

var (
	vulkanGPUNamesCache map[string]string
	vulkanGPUNamesDone  bool
)

func getAMDGPUs() []GPUStats {
	var gpus []GPUStats

	if !vulkanGPUNamesDone {
		vulkanGPUNamesCache = getVulkanGPUNames()
		vulkanGPUNamesDone = true
	}
	gpuNamesByID := vulkanGPUNamesCache

	for cardNum := 0; cardNum < 4; cardNum++ {
		basePath := fmt.Sprintf("/sys/class/drm/card%d/device", cardNum)

		if _, err := os.Stat(basePath + "/gpu_busy_percent"); err != nil {
			continue
		}

		gpu := GPUStats{}

		// Read device ID from sysfs and match to Vulkan GPU name
		if data, err := os.ReadFile(basePath + "/device"); err == nil {
			deviceID := strings.TrimSpace(string(data))
			// Remove 0x prefix and convert to uppercase
			deviceID = strings.TrimPrefix(deviceID, "0x")
			deviceID = strings.ToUpper(deviceID)

			if name, ok := gpuNamesByID[deviceID]; ok {
				gpu.Name = name
			} else {
				gpu.Name = fmt.Sprintf("AMD GPU (card%d)", cardNum)
			}
		} else {
			gpu.Name = fmt.Sprintf("AMD GPU (card%d)", cardNum)
		}

		if data, err := os.ReadFile(basePath + "/gpu_busy_percent"); err == nil {
			if val, err := strconv.ParseFloat(strings.TrimSpace(string(data)), 64); err == nil {
				gpu.UtilizationGPU = val
			}
		}

		if data, err := os.ReadFile(basePath + "/mem_busy_percent"); err == nil {
			if val, err := strconv.ParseFloat(strings.TrimSpace(string(data)), 64); err == nil {
				gpu.UtilizationMemory = val
			}
		}

		if data, err := os.ReadFile(basePath + "/mem_info_vram_used"); err == nil {
			if val, err := strconv.ParseUint(strings.TrimSpace(string(data)), 10, 64); err == nil {
				gpu.MemoryUsed = val
			}
		}

		if data, err := os.ReadFile(basePath + "/mem_info_vram_total"); err == nil {
			if val, err := strconv.ParseUint(strings.TrimSpace(string(data)), 10, 64); err == nil {
				gpu.MemoryTotal = val
			}
		}

		if gpu.MemoryTotal > 0 {
			gpu.MemoryPercent = float64(gpu.MemoryUsed) / float64(gpu.MemoryTotal) * 100
		}

		tempFiles, _ := os.ReadDir(basePath + "/hwmon")
		for _, hwmon := range tempFiles {
			if strings.HasPrefix(hwmon.Name(), "hwmon") {
				tempPath := fmt.Sprintf("%s/hwmon/%s/temp1_input", basePath, hwmon.Name())
				if data, err := os.ReadFile(tempPath); err == nil {
					if val, err := strconv.ParseFloat(strings.TrimSpace(string(data)), 64); err == nil {
						gpu.Temperature = val / 1000
						break
					}
				}
			}
		}

		gpus = append(gpus, gpu)
	}

	return gpus
}

// ---------------------------------------------------------------------------
// Formatting helpers.
// ---------------------------------------------------------------------------

// FormatBytes converts bytes to human-readable format.
func FormatBytes(bytes uint64) string {
	const unit = 1024
	if bytes < unit {
		return fmt.Sprintf("%d B", bytes)
	}
	div, exp := uint64(unit), 0
	for n := bytes / unit; n >= unit; n /= unit {
		div *= unit
		exp++
	}
	return fmt.Sprintf("%.1f %ciB", float64(bytes)/float64(div), "KMGTPE"[exp])
}

// FormatUptime converts seconds to human-readable uptime.
func FormatUptime(seconds uint64) string {
	days := seconds / 86400
	hours := (seconds % 86400) / 3600
	minutes := (seconds % 3600) / 60

	if days > 0 {
		return fmt.Sprintf("%dd %dh %dm", days, hours, minutes)
	}
	if hours > 0 {
		return fmt.Sprintf("%dh %dm", hours, minutes)
	}
	return fmt.Sprintf("%dm", minutes)
}

// GetLoadAverage retrieves system load averages.
func GetLoadAverage() ([]float64, error) {
	file, err := os.Open("/proc/loadavg")
	if err != nil {
		return nil, err
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	if !scanner.Scan() {
		return nil, fmt.Errorf("failed to read /proc/loadavg")
	}

	fields := strings.Fields(scanner.Text())
	if len(fields) < 3 {
		return nil, fmt.Errorf("unexpected format in /proc/loadavg")
	}

	load := make([]float64, 3)
	for i := 0; i < 3; i++ {
		val, err := strconv.ParseFloat(fields[i], 64)
		if err != nil {
			return nil, err
		}
		load[i] = val
	}

	return load, nil
}

// ---------------------------------------------------------------------------
// Network stats.
// ---------------------------------------------------------------------------

// netRateSample holds previous byte counts and history for rate calculation.
type netRateSample struct {
	bytesSent uint64
	bytesRecv uint64
	rxHistory []uint64
	txHistory []uint64
}

// netIfaceStatic holds the non-changing fields of a network interface.
type netIfaceStatic struct {
	name   string
	ifType NetworkInterfaceType
	hwAddr string
	addrs  []string
}

var (
	netRateCache      = map[string]*netRateSample{}
	netRateCacheMu    sync.Mutex
	netIfaceCache     []netIfaceStatic // cached on first call
	netIfaceCacheMu   sync.Mutex
	netIfaceCacheDone bool
)

const netHistoryLen = 90

// classifyInterface returns the type of a network interface based on its name and kernel state.
func classifyInterface(name string) NetworkInterfaceType {
	switch {
	case strings.HasPrefix(name, "wg"):
		return NetTypeWireguard
	case strings.HasPrefix(name, "tun") || strings.HasPrefix(name, "tap"):
		return NetTypeVPN
	case strings.HasPrefix(name, "docker") || strings.HasPrefix(name, "veth"):
		return NetTypeDocker
	case strings.HasPrefix(name, "br-") || strings.HasPrefix(name, "virbr"):
		return NetTypeBridge
	case strings.HasPrefix(name, "wlan") || strings.HasPrefix(name, "wlp") || strings.HasPrefix(name, "wl"):
		return NetTypeWifi
	}
	// VLAN: kernel exposes /proc/net/vlan/<ifname> for 802.1q interfaces.
	if _, err := os.Stat("/proc/net/vlan/" + name); err == nil {
		return NetTypeVLAN
	}
	switch {
	case strings.HasPrefix(name, "eth") || strings.HasPrefix(name, "enp") || strings.HasPrefix(name, "eno") || strings.HasPrefix(name, "ens"):
		return NetTypeEthernet
	default:
		return NetTypeOther
	}
}

// loadNetIfaceCacheOnce populates netIfaceCache from gopsutil on first call.
// Subsequent calls skip the expensive netlink Interfaces() scan.
func loadNetIfaceCacheOnce() {
	netIfaceCacheMu.Lock()
	defer netIfaceCacheMu.Unlock()
	if netIfaceCacheDone {
		return
	}
	interfaces, err := net.Interfaces()
	if err != nil {
		netIfaceCacheDone = true
		return
	}
	for _, iface := range interfaces {
		if iface.Name == "lo" || len(iface.Flags) == 0 {
			continue
		}
		var addrs []string
		for _, addr := range iface.Addrs {
			addrs = append(addrs, addr.Addr)
		}
		netIfaceCache = append(netIfaceCache, netIfaceStatic{
			name:   iface.Name,
			ifType: classifyInterface(iface.Name),
			hwAddr: iface.HardwareAddr,
			addrs:  addrs,
		})
	}
	netIfaceCacheDone = true
}

// readNetDevIO parses /proc/net/dev for per-interface IO counters without
// invoking gopsutil (avoids repeated netlink calls).
func readNetDevIO() (map[string][4]uint64, error) {
	f, err := os.Open("/proc/net/dev")
	if err != nil {
		return nil, err
	}
	defer f.Close()

	// Read entire file into a small fixed buffer.
	var buf [4096]byte
	n, _ := f.Read(buf[:])
	data := buf[:n]

	result := make(map[string][4]uint64, 8)

	// Skip 2 header lines.
	lineStart := 0
	skipped := 0
	for i := 0; i < n; i++ {
		if data[i] == '\n' {
			skipped++
			lineStart = i + 1
			if skipped == 2 {
				break
			}
		}
	}

	for lineStart < n {
		// Find end of line.
		lineEnd := lineStart
		for lineEnd < n && data[lineEnd] != '\n' {
			lineEnd++
		}
		line := data[lineStart:lineEnd]

		// Find colon separating interface name.
		colon := -1
		for i, c := range line {
			if c == ':' {
				colon = i
				break
			}
		}
		if colon < 0 {
			lineStart = lineEnd + 1
			continue
		}
		// Trim name.
		nameBytes := line[:colon]
		nameStart := 0
		for nameStart < len(nameBytes) && nameBytes[nameStart] == ' ' {
			nameStart++
		}
		name := string(nameBytes[nameStart:])

		// Parse fields after colon: [bytesRecv packetsRecv errs drop fifo frame comp multi]
		//                            [bytesSent packetsSent errs drop fifo colls carrier comp]
		// We want: bytesRecv[0], packetsRecv[1], bytesSent[8], packetsSent[9]
		p := colon + 1
		var vals [10]uint64
		fi := 0
		for fi < 10 && p < len(line) {
			for p < len(line) && line[p] == ' ' {
				p++
			}
			var v uint64
			for p < len(line) && line[p] >= '0' && line[p] <= '9' {
				v = v*10 + uint64(line[p]-'0')
				p++
			}
			vals[fi] = v
			fi++
		}
		result[name] = [4]uint64{vals[0], vals[1], vals[8], vals[9]}
		// indices: bytesRecv, packetsRecv, bytesSent, packetsSent

		lineStart = lineEnd + 1
	}
	return result, nil
}

// readIfaceOperState reads the operational state for a single interface from /sys/class/net.
func readIfaceOperState(name string) string {
	data, err := os.ReadFile("/sys/class/net/" + name + "/operstate")
	if err != nil {
		return "down"
	}
	if strings.TrimSpace(string(data)) == "up" {
		return "up"
	}
	return "down"
}

// readDefaultGateway parses /proc/net/route and returns the default IPv4 gateway
// for the given interface, or the global default gateway if none matches.
func readDefaultGateway(ifaceName string) string {
	data, err := os.ReadFile("/proc/net/route")
	if err != nil {
		return ""
	}
	type routeEntry struct {
		iface   string
		gateway string
	}
	var ifaceGW, globalGW string
	for i, line := range strings.Split(string(data), "\n") {
		if i == 0 || strings.TrimSpace(line) == "" {
			continue // skip header
		}
		fields := strings.Fields(line)
		if len(fields) < 3 {
			continue
		}
		dest := fields[1]
		gwHex := fields[2]
		if dest != "00000000" {
			continue // only default route
		}
		// Gateway is 32-bit little-endian hex.
		gwBytes, err := strconv.ParseUint(gwHex, 16, 32)
		if err != nil {
			continue
		}
		b := [4]byte{byte(gwBytes), byte(gwBytes >> 8), byte(gwBytes >> 16), byte(gwBytes >> 24)}
		gw := fmt.Sprintf("%d.%d.%d.%d", b[0], b[1], b[2], b[3])
		if gw == "0.0.0.0" {
			continue
		}
		if fields[0] == ifaceName {
			ifaceGW = gw
		} else if globalGW == "" {
			globalGW = gw
		}
	}
	if ifaceGW != "" {
		return ifaceGW
	}
	return globalGW
}

// readDNSServers parses /etc/resolv.conf and returns nameserver addresses.
// Result is cached after the first successful read.
var (
	dnsCache     []string
	dnsCacheMu   sync.Mutex
	dnsCacheDone bool
)

func readDNSServers() []string {
	dnsCacheMu.Lock()
	defer dnsCacheMu.Unlock()
	if dnsCacheDone {
		return dnsCache
	}
	data, err := os.ReadFile("/etc/resolv.conf")
	if err != nil {
		dnsCacheDone = true
		return nil
	}
	for _, line := range strings.Split(string(data), "\n") {
		line = strings.TrimSpace(line)
		if strings.HasPrefix(line, "nameserver") {
			fields := strings.Fields(line)
			if len(fields) >= 2 {
				dnsCache = append(dnsCache, fields[1])
			}
		}
	}
	dnsCacheDone = true
	return dnsCache
}

// detectIPSource returns "dhcp" if a DHCP lease exists for the interface, "static" otherwise.
func detectIPSource(ifaceName string) string {
	// systemd-networkd leases.
	leaseDir := "/run/systemd/netif/leases"
	entries, err := os.ReadDir(leaseDir)
	if err == nil {
		for _, e := range entries {
			if strings.Contains(e.Name(), ifaceName) {
				return "dhcp"
			}
		}
	}
	// dhcpcd leases.
	if _, err := os.Stat("/run/dhcpcd/pid"); err == nil {
		if _, err := os.Stat(fmt.Sprintf("/var/lib/dhcpcd/%s.lease", ifaceName)); err == nil {
			return "dhcp"
		}
	}
	// NetworkManager leases.
	nmLeaseDir := "/var/lib/NetworkManager"
	nmEntries, err := os.ReadDir(nmLeaseDir)
	if err == nil {
		for _, e := range nmEntries {
			name := e.Name()
			if strings.HasPrefix(name, "dhclient-") && strings.Contains(name, ifaceName) {
				return "dhcp"
			}
			if strings.HasSuffix(name, ".lease") && strings.Contains(name, ifaceName) {
				return "dhcp"
			}
		}
	}
	// Internal dhclient leases.
	if _, err := os.Stat(fmt.Sprintf("/var/lib/dhclient/dhclient-%s.leases", ifaceName)); err == nil {
		return "dhcp"
	}
	return "static"
}

// readVLANInfo reads the VLAN ID and parent interface from /proc/net/vlan/<ifname>.
// Returns (0, "") if the file does not exist or cannot be parsed.
func readVLANInfo(ifaceName string) (vlanID int, parent string) {
	data, err := os.ReadFile("/proc/net/vlan/" + ifaceName)
	if err != nil {
		return 0, ""
	}
	// Format example:
	// eth0.100  VID: 100	 REORDER_HDR: 1  dev->priv_flags: 1
	// 	 Device: eth0
	for _, line := range strings.Split(string(data), "\n") {
		line = strings.TrimSpace(line)
		if strings.HasPrefix(line, "Device:") {
			fields := strings.Fields(line)
			if len(fields) >= 2 {
				parent = fields[1]
			}
		}
		if idx := strings.Index(line, "VID:"); idx >= 0 {
			rest := strings.TrimSpace(line[idx+4:])
			fields := strings.Fields(rest)
			if len(fields) > 0 {
				if id, err := strconv.Atoi(fields[0]); err == nil {
					vlanID = id
				}
			}
		}
	}
	return vlanID, parent
}

// GetNetworkStats retrieves network interface statistics with transfer rates and history.
// Interface metadata (name, type, hardware address, IPs) is cached after the first call.
// Only /proc/net/dev is re-read each tick.
func GetNetworkStats() ([]NetworkStats, error) {
	loadNetIfaceCacheOnce()

	ioMap, err := readNetDevIO()
	if err != nil {
		return nil, err
	}

	netRateCacheMu.Lock()
	defer netRateCacheMu.Unlock()

	netIfaceCacheMu.Lock()
	ifaces := netIfaceCache
	netIfaceCacheMu.Unlock()

	stats := make([]NetworkStats, 0, len(ifaces))

	for _, iface := range ifaces {
		vlanID, vlanParent := 0, ""
		if iface.ifType == NetTypeVLAN {
			vlanID, vlanParent = readVLANInfo(iface.name)
		}
		stat := NetworkStats{
			Interface:    iface.name,
			Type:         iface.ifType,
			Status:       readIfaceOperState(iface.name),
			HardwareAddr: iface.hwAddr,
			IPAddresses:  iface.addrs,
			Gateway:      readDefaultGateway(iface.name),
			DNSServers:   readDNSServers(),
			IPSource:     detectIPSource(iface.name),
			VLANID:       vlanID,
			VLANParent:   vlanParent,
		}

		if io, ok := ioMap[iface.name]; ok {
			// io: [bytesRecv, packetsRecv, bytesSent, packetsSent]
			stat.BytesRecv = io[0]
			stat.PacketsRecv = io[1]
			stat.BytesSent = io[2]
			stat.PacketsSent = io[3]

			prev, hasPrev := netRateCache[iface.name]
			if hasPrev {
				txDelta := uint64(0)
				rxDelta := uint64(0)
				if stat.BytesSent >= prev.bytesSent {
					txDelta = stat.BytesSent - prev.bytesSent
				}
				if stat.BytesRecv >= prev.bytesRecv {
					rxDelta = stat.BytesRecv - prev.bytesRecv
				}
				stat.TxRate = txDelta / 2
				stat.RxRate = rxDelta / 2

				rxHist := append(prev.rxHistory, stat.RxRate)
				txHist := append(prev.txHistory, stat.TxRate)
				if len(rxHist) > netHistoryLen {
					rxHist = rxHist[len(rxHist)-netHistoryLen:]
				}
				if len(txHist) > netHistoryLen {
					txHist = txHist[len(txHist)-netHistoryLen:]
				}
				stat.RxHistory = rxHist
				stat.TxHistory = txHist
			}

			netRateCache[iface.name] = &netRateSample{
				bytesSent: stat.BytesSent,
				bytesRecv: stat.BytesRecv,
				rxHistory: stat.RxHistory,
				txHistory: stat.TxHistory,
			}
		}

		stats = append(stats, stat)
	}

	return stats, nil
}

// ---------------------------------------------------------------------------
// Temperature stats — hwmon sensor map cached after first scan.
// ---------------------------------------------------------------------------

type hwmonEntry struct {
	path     string
	category string // "cpu", "gpu", "mem"
}

var (
	hwmonCache     []hwmonEntry
	hwmonCacheMu   sync.Mutex
	hwmonCacheDone bool
)

// GetTemperatureStats retrieves temperature sensor readings for CPU, GPU, and Memory.
// Caches the hwmon sensor path→category mapping after the first scan.
func GetTemperatureStats() (*TemperatureStats, error) {
	hwmonCacheMu.Lock()
	if !hwmonCacheDone {
		hwmonCache = buildHwmonCache()
		hwmonCacheDone = true
	}
	entries := hwmonCache
	hwmonCacheMu.Unlock()

	temps := &TemperatureStats{}
	for _, entry := range entries {
		data, err := os.ReadFile(entry.path + "/temp1_input")
		if err != nil {
			continue
		}
		tempMilliC, err := strconv.ParseFloat(strings.TrimSpace(string(data)), 64)
		if err != nil {
			continue
		}
		temp := tempMilliC / 1000.0

		switch entry.category {
		case "cpu":
			if temps.CPUTemp == 0 || temp > temps.CPUTemp {
				temps.CPUTemp = temp
			}
		case "gpu":
			if temps.GPUTemp == 0 || temp > temps.GPUTemp {
				temps.GPUTemp = temp
			}
		case "mem":
			if temps.MemoryTemp == 0 || temp > temps.MemoryTemp {
				temps.MemoryTemp = temp
			}
		}
	}

	// Append CPU temp to rolling history.
	cpuTempHistoryMu.Lock()
	cpuTempHistoryData = appendGPUHistory(cpuTempHistoryData, temps.CPUTemp)
	temps.CPUTempHistory = append([]float64(nil), cpuTempHistoryData...)
	cpuTempHistoryMu.Unlock()

	return temps, nil
}

// buildHwmonCache scans /sys/class/hwmon once and builds path→category pairs.
func buildHwmonCache() []hwmonEntry {
	hwmonDirs, err := os.ReadDir("/sys/class/hwmon")
	if err != nil {
		return nil
	}

	var entries []hwmonEntry
	for _, hwmon := range hwmonDirs {
		basePath := fmt.Sprintf("/sys/class/hwmon/%s", hwmon.Name())

		nameBytes, err := os.ReadFile(basePath + "/name")
		if err != nil {
			continue
		}
		sensorName := strings.TrimSpace(string(nameBytes))

		// Confirm temp1_input exists before caching.
		if _, err := os.Stat(basePath + "/temp1_input"); err != nil {
			continue
		}

		var category string
		switch {
		case sensorName == "k10temp" || sensorName == "coretemp" || strings.Contains(sensorName, "cpu"):
			category = "cpu"
		case sensorName == "amdgpu" || sensorName == "nouveau" || strings.Contains(sensorName, "gpu"):
			category = "gpu"
		case strings.Contains(sensorName, "spd") || strings.Contains(sensorName, "ddr"):
			category = "mem"
		default:
			continue // not a sensor we care about
		}

		entries = append(entries, hwmonEntry{path: basePath, category: category})
	}
	return entries
}

// ---------------------------------------------------------------------------
// Process stats — /proc/stat read once via readProcStat above.
// ---------------------------------------------------------------------------

// procTicks holds the last-seen CPU tick count for a PID, used to compute delta.
type procTicks struct {
	total uint64
}

var (
	prevProcTicks  = map[int32]procTicks{}
	prevTotalTicks uint64
	procTicksMu    sync.Mutex
)

// readProcStatTotalTicks reads only the aggregate CPU ticks from /proc/stat.
// Used by GetTopProcesses when GetCPUStats has already consumed the /proc/stat
// delta for this tick via readProcStat().
func readProcStatTotalTicks() (uint64, error) {
	f, err := os.Open("/proc/stat")
	if err != nil {
		return 0, err
	}
	defer f.Close()

	scanner := bufio.NewScanner(f)
	if !scanner.Scan() {
		return 0, fmt.Errorf("empty /proc/stat")
	}

	fields := strings.Fields(scanner.Text())
	var total uint64
	for _, field := range fields[1:] {
		v, err := strconv.ParseUint(field, 10, 64)
		if err != nil {
			break
		}
		total += v
	}
	return total, nil
}

// readSingleProcStat reads utime+stime, name, and RSS for a single PID from
// /proc/<pid>/stat only — RSS is at field index 23 (0-based after closing paren),
// eliminating the separate /proc/<pid>/statm read.
func readSingleProcStat(pid int32) (name string, ticks uint64, rssPages uint64, ok bool) {
	var buf [512]byte
	path := fmt.Sprintf("/proc/%d/stat", pid)
	f, err := os.Open(path)
	if err != nil {
		return
	}
	n, _ := f.Read(buf[:])
	f.Close()
	if n == 0 {
		return
	}
	raw := buf[:n]

	// Find process name between first '(' and last ')'.
	start := -1
	end := -1
	for i := 0; i < n; i++ {
		if raw[i] == '(' {
			start = i
			break
		}
	}
	for i := n - 1; i >= 0; i-- {
		if raw[i] == ')' {
			end = i
			break
		}
	}
	if start < 0 || end < 0 || end <= start {
		return
	}
	name = string(raw[start+1 : end])

	// Parse space-separated fields after ')' without allocating via strings.Fields.
	// Fields (0-indexed): 0=state, 1=ppid, ..., 11=utime, 12=stime, ..., 23=rss
	pos := end + 2 // skip ') '
	if pos >= n {
		return
	}

	const (
		fieldUtime = 11
		fieldStime = 12
		fieldRSS   = 21 // rss in pages: 0-indexed after ')'; cat -n shows it as line 22 = index 21
	)

	var utime, stime uint64
	fieldIdx := 0
	i := pos
	for i < n && fieldIdx <= fieldRSS {
		// Skip spaces.
		for i < n && raw[i] == ' ' {
			i++
		}
		if i >= n {
			break
		}
		// Read token.
		j := i
		for j < n && raw[j] != ' ' && raw[j] != '\n' {
			j++
		}
		if fieldIdx == fieldUtime {
			utime = parseUint64Bytes(raw[i:j])
		} else if fieldIdx == fieldStime {
			stime = parseUint64Bytes(raw[i:j])
		} else if fieldIdx == fieldRSS {
			rssPages = parseUint64Bytes(raw[i:j])
		}
		i = j
		fieldIdx++
	}

	if fieldIdx <= fieldStime {
		return // didn't reach stime
	}
	ticks = utime + stime
	ok = true
	return
}

// parseUint64Bytes parses a decimal uint64 from a byte slice without allocating.
func parseUint64Bytes(b []byte) uint64 {
	var v uint64
	for _, c := range b {
		if c < '0' || c > '9' {
			break
		}
		v = v*10 + uint64(c-'0')
	}
	return v
}

// GetTopProcesses retrieves top processes by CPU usage.
// Reads /proc directly to avoid the overhead of gopsutil's per-process syscall storm.
// MemTotal and page size are read once and cached; /proc/stat total ticks come from
// the shared prevTotalTicks updated by GetCPUStats().
func GetTopProcesses(limit int) ([]ProcessInfo, error) {
	// Use the total ticks already updated by GetCPUStats() this tick.
	// Fall back to reading /proc/stat directly if needed (first call or standalone use).
	procStatMu.Lock()
	currentTotal := prevTotalTicks
	procStatMu.Unlock()

	if currentTotal == 0 {
		var err error
		currentTotal, err = readProcStatTotalTicks()
		if err != nil {
			return nil, err
		}
	}

	entries, err := os.ReadDir("/proc")
	if err != nil {
		return nil, err
	}

	pageSize := getPageSize()
	memTotal := getMemTotalBytes()

	procTicksMu.Lock()
	defer procTicksMu.Unlock()

	// prevTotalTicks as of last GetTopProcesses call.
	deltaTotalTicks := currentTotal - prevProcTicksTotal
	if deltaTotalTicks == 0 {
		deltaTotalTicks = 1
	}

	var processList []ProcessInfo
	currentPIDs := map[int32]struct{}{}

	for _, entry := range entries {
		if !entry.IsDir() {
			continue
		}
		pid, err := strconv.ParseInt(entry.Name(), 10, 32)
		if err != nil {
			continue
		}
		p := int32(pid)
		currentPIDs[p] = struct{}{}

		name, ticks, rssPages, ok := readSingleProcStat(p)
		if !ok {
			continue
		}

		prev := prevProcTicks[p]
		deltaProcTicks := ticks - prev.total
		prevProcTicks[p] = procTicks{total: ticks}

		cpuPercent := float64(deltaProcTicks) / float64(deltaTotalTicks) * 100.0
		rssBytes := rssPages * pageSize
		memPercent := float32(0)
		if memTotal > 0 {
			memPercent = float32(rssBytes) / float32(memTotal) * 100.0
		}

		processList = append(processList, ProcessInfo{
			PID:         p,
			Name:        name,
			CPUPercent:  cpuPercent,
			MemPercent:  memPercent,
			MemoryBytes: rssBytes,
		})
	}

	// Evict stale PIDs.
	for p := range prevProcTicks {
		if _, alive := currentPIDs[p]; !alive {
			delete(prevProcTicks, p)
		}
	}

	prevProcTicksTotal = currentTotal

	sort.Slice(processList, func(i, j int) bool {
		return processList[i].CPUPercent > processList[j].CPUPercent
	})

	if len(processList) > limit {
		processList = processList[:limit]
	}

	return processList, nil
}

// prevProcTicksTotal is the total CPU ticks as of the last GetTopProcesses call.
var prevProcTicksTotal uint64
