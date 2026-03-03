package system

import (
	"bufio"
	"fmt"
	"os"
	"os/exec"
	"sort"
	"strconv"
	"strings"
	"syscall"
	"time"

	"github.com/shirou/gopsutil/v4/cpu"
	"github.com/shirou/gopsutil/v4/host"
	"github.com/shirou/gopsutil/v4/mem"
	"github.com/shirou/gopsutil/v4/net"
	"github.com/shirou/gopsutil/v4/process"
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

// NetworkStats represents network interface statistics.
type NetworkStats struct {
	Interface   string
	Status      string
	IPAddress   string
	BytesSent   uint64
	BytesRecv   uint64
	PacketsSent uint64
	PacketsRecv uint64
}

// TemperatureStats represents categorized temperature sensor readings.
type TemperatureStats struct {
	CPUTemp    float64
	GPUTemp    float64
	MemoryTemp float64
}

// ProcessInfo represents a running process.
type ProcessInfo struct {
	PID         int32
	Name        string
	CPUPercent  float64
	MemPercent  float32
	MemoryBytes uint64
}

// GetCPUStats retrieves current CPU usage statistics.
func GetCPUStats() (*CPUStats, error) {
	// Use a shorter interval (100ms) to reduce blocking time
	perCore, err := cpu.Percent(100*time.Millisecond, true)
	if err != nil {
		return nil, fmt.Errorf("failed to get per-core CPU percent: %w", err)
	}

	// Get logical cores (includes hyperthreading)
	logicalCores, err := cpu.Counts(true)
	if err != nil {
		return nil, fmt.Errorf("failed to get logical CPU count: %w", err)
	}

	// Get physical cores (excludes hyperthreading)
	physicalCores, err := cpu.Counts(false)
	if err != nil {
		return nil, fmt.Errorf("failed to get physical CPU count: %w", err)
	}

	// Get CPU info for model name and socket count
	info, err := cpu.Info()
	if err != nil {
		return nil, fmt.Errorf("failed to get CPU info: %w", err)
	}

	modelName := "Unknown"
	cpuCount := 1
	if len(info) > 0 {
		modelName = info[0].ModelName

		// Count unique physical IDs to determine CPU socket count
		physicalIDs := make(map[string]bool)
		for _, cpuInfo := range info {
			physicalIDs[cpuInfo.PhysicalID] = true
		}
		cpuCount = len(physicalIDs)
	}

	// Calculate overall from per-core
	var total float64
	for _, usage := range perCore {
		total += usage
	}
	overall := total / float64(len(perCore))

	return &CPUStats{
		UsagePercent:  overall,
		PhysicalCores: physicalCores,
		LogicalCores:  logicalCores,
		PerCoreUsage:  perCore,
		ModelName:     modelName,
		CPUCount:      cpuCount,
	}, nil
}

// GetMemoryStats retrieves current memory usage statistics.
func GetMemoryStats() (*MemoryStats, error) {
	vmem, err := mem.VirtualMemory()
	if err != nil {
		return nil, fmt.Errorf("failed to get virtual memory: %w", err)
	}

	swap, err := mem.SwapMemory()
	if err != nil {
		return nil, fmt.Errorf("failed to get swap memory: %w", err)
	}

	return &MemoryStats{
		Total:       vmem.Total,
		Used:        vmem.Used,
		Available:   vmem.Available,
		UsedPercent: vmem.UsedPercent,
		SwapTotal:   swap.Total,
		SwapUsed:    swap.Used,
		SwapPercent: swap.UsedPercent,
	}, nil
}

// GetDiskStats retrieves disk usage for all mounted filesystems.
func GetDiskStats() ([]DiskStats, error) {
	var stats []DiskStats

	// Read /proc/mounts to get all mounted filesystems
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

		// Skip virtual filesystems and duplicates
		if shouldSkipFilesystem(device, mountPoint, fsType) {
			continue
		}

		// Skip duplicates (same mount point)
		if seen[mountPoint] {
			continue
		}
		seen[mountPoint] = true

		var stat syscall.Statfs_t
		if err := syscall.Statfs(mountPoint, &stat); err != nil {
			continue
		}

		total := stat.Blocks * uint64(stat.Bsize)
		// Skip zero-size filesystems
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
	// Skip virtual/pseudo filesystems
	skipFsTypes := map[string]bool{
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

	if skipFsTypes[fsType] {
		return true
	}

	// Skip non-block devices (unless they're network mounts or special cases)
	if !strings.HasPrefix(device, "/dev/") &&
		!strings.HasPrefix(device, "//") && // SMB
		!strings.Contains(device, ":") { // NFS
		return true
	}

	// Skip certain mount points
	skipMounts := map[string]bool{
		"/proc":     true,
		"/sys":      true,
		"/dev":      true,
		"/run":      true,
		"/tmp":      true,
		"/var/tmp":  true,
		"/dev/shm":  true,
		"/run/user": true,
	}

	if skipMounts[mountPoint] {
		return true
	}

	// Skip /run/user/* mounts
	if strings.HasPrefix(mountPoint, "/run/user/") {
		return true
	}

	return false
}

// GetSystemInfo retrieves general system information.
func GetSystemInfo() (*SystemInfo, error) {
	info, err := host.Info()
	if err != nil {
		return nil, fmt.Errorf("failed to get host info: %w", err)
	}

	version, err := getNixOSVersion()
	if err != nil {
		version = "unknown"
	}

	return &SystemInfo{
		Hostname:      info.Hostname,
		Uptime:        info.Uptime,
		OS:            info.OS,
		Platform:      info.Platform,
		KernelVersion: info.KernelVersion,
		NixOSVersion:  version,
	}, nil
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
		// Look for BUILD_ID="26.05.20260227.dd9b079"
		if strings.HasPrefix(line, "BUILD_ID=") {
			parts := strings.SplitN(line, "=", 2)
			if len(parts) == 2 {
				// Remove quotes
				buildID := strings.Trim(parts[1], "\"")
				return buildID, nil
			}
		}
	}

	if err := scanner.Err(); err != nil {
		return "", err
	}

	return "unknown", nil
}

// GetGPUStats retrieves GPU statistics.
// Tries nvidia-smi (NVIDIA), rocm-smi (AMD), or falls back to basic detection.
func GetGPUStats() ([]GPUStats, error) {
	// Try NVIDIA first
	if gpus := getNVIDIAGPUs(); len(gpus) > 0 {
		return gpus, nil
	}

	// Try AMD
	if gpus := getAMDGPUs(); len(gpus) > 0 {
		return gpus, nil
	}

	// No GPU monitoring available
	return nil, nil
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

		memUsedBytes := memUsed * 1024 * 1024 // Convert MiB to bytes
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
func getVulkanGPUNames() []string {
	var names []string

	cmd := exec.Command("vulkaninfo")
	output, err := cmd.Output()
	if err != nil {
		return names
	}

	scanner := bufio.NewScanner(strings.NewReader(string(output)))
	for scanner.Scan() {
		line := scanner.Text()
		if strings.Contains(line, "deviceName") && strings.Contains(line, "=") {
			// Format: "	deviceName        = AMD Radeon RX 9070 XT (RADV GFX1201)"
			parts := strings.SplitN(line, "=", 2)
			if len(parts) == 2 {
				name := strings.TrimSpace(parts[1])
				// Remove driver info in parentheses
				if idx := strings.Index(name, "("); idx != -1 {
					name = strings.TrimSpace(name[:idx])
				}
				// Skip llvmpipe (software renderer)
				if !strings.Contains(name, "llvmpipe") {
					names = append(names, name)
				}
			}
		}
	}

	return names
}

func getAMDGPUs() []GPUStats {
	var gpus []GPUStats

	// Get GPU names from vulkaninfo once (more efficient and accurate)
	gpuNames := getVulkanGPUNames()

	// AMD GPUs expose stats via sysfs under /sys/class/drm/cardN/device/
	// Check common card indices (card0, card1, etc.)
	gpuIndex := 0
	for cardNum := 0; cardNum < 4; cardNum++ {
		basePath := fmt.Sprintf("/sys/class/drm/card%d/device", cardNum)

		// Check if this is an AMD GPU by looking for gpu_busy_percent
		if _, err := os.Stat(basePath + "/gpu_busy_percent"); err != nil {
			continue
		}

		gpu := GPUStats{}

		// Use vulkan-detected name if available
		if gpuIndex < len(gpuNames) {
			gpu.Name = gpuNames[gpuIndex]
		} else {
			gpu.Name = fmt.Sprintf("AMD GPU (card%d)", cardNum)
		}
		gpuIndex++

		// Read GPU utilization
		if data, err := os.ReadFile(basePath + "/gpu_busy_percent"); err == nil {
			if val, err := strconv.ParseFloat(strings.TrimSpace(string(data)), 64); err == nil {
				gpu.UtilizationGPU = val
			}
		}

		// Read memory utilization
		if data, err := os.ReadFile(basePath + "/mem_busy_percent"); err == nil {
			if val, err := strconv.ParseFloat(strings.TrimSpace(string(data)), 64); err == nil {
				gpu.UtilizationMemory = val
			}
		}

		// Read VRAM usage
		if data, err := os.ReadFile(basePath + "/mem_info_vram_used"); err == nil {
			if val, err := strconv.ParseUint(strings.TrimSpace(string(data)), 10, 64); err == nil {
				gpu.MemoryUsed = val
			}
		}

		// Read VRAM total
		if data, err := os.ReadFile(basePath + "/mem_info_vram_total"); err == nil {
			if val, err := strconv.ParseUint(strings.TrimSpace(string(data)), 10, 64); err == nil {
				gpu.MemoryTotal = val
			}
		}

		// Calculate memory percentage
		if gpu.MemoryTotal > 0 {
			gpu.MemoryPercent = float64(gpu.MemoryUsed) / float64(gpu.MemoryTotal) * 100
		}

		// Read temperature (in millidegrees Celsius)
		tempFiles, _ := os.ReadDir(basePath + "/hwmon")
		for _, hwmon := range tempFiles {
			if strings.HasPrefix(hwmon.Name(), "hwmon") {
				tempPath := fmt.Sprintf("%s/hwmon/%s/temp1_input", basePath, hwmon.Name())
				if data, err := os.ReadFile(tempPath); err == nil {
					if val, err := strconv.ParseFloat(strings.TrimSpace(string(data)), 64); err == nil {
						gpu.Temperature = val / 1000 // Convert millidegrees to degrees
						break
					}
				}
			}
		}

		gpus = append(gpus, gpu)
	}

	return gpus
}

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

// GetNetworkStats retrieves network interface statistics.
func GetNetworkStats() ([]NetworkStats, error) {
	var stats []NetworkStats

	interfaces, err := net.Interfaces()
	if err != nil {
		return nil, err
	}

	ioCounters, err := net.IOCounters(true)
	if err != nil {
		return nil, err
	}

	// Create a map for quick lookup
	ioMap := make(map[string]net.IOCountersStat)
	for _, io := range ioCounters {
		ioMap[io.Name] = io
	}

	for _, iface := range interfaces {
		// Skip loopback and down interfaces
		if iface.Name == "lo" || len(iface.Flags) == 0 {
			continue
		}

		status := "down"
		if len(iface.Flags) > 0 && strings.Contains(strings.Join(iface.Flags, ","), "up") {
			status = "up"
		}

		ipAddr := ""
		if len(iface.Addrs) > 0 {
			// Get first IPv4 address
			for _, addr := range iface.Addrs {
				if !strings.Contains(addr.Addr, ":") { // Skip IPv6
					ipAddr = addr.Addr
					break
				}
			}
		}

		stat := NetworkStats{
			Interface: iface.Name,
			Status:    status,
			IPAddress: ipAddr,
		}

		if io, ok := ioMap[iface.Name]; ok {
			stat.BytesSent = io.BytesSent
			stat.BytesRecv = io.BytesRecv
			stat.PacketsSent = io.PacketsSent
			stat.PacketsRecv = io.PacketsRecv
		}

		stats = append(stats, stat)
	}

	return stats, nil
}

// GetTemperatureStats retrieves temperature sensor readings for CPU, GPU, and Memory.
func GetTemperatureStats() (*TemperatureStats, error) {
	temps := &TemperatureStats{}

	// Try to read from hwmon sysfs
	hwmonDirs, err := os.ReadDir("/sys/class/hwmon")
	if err != nil {
		return temps, nil // Not an error if hwmon doesn't exist
	}

	for _, hwmon := range hwmonDirs {
		basePath := fmt.Sprintf("/sys/class/hwmon/%s", hwmon.Name())

		// Read sensor name
		nameBytes, err := os.ReadFile(basePath + "/name")
		if err != nil {
			continue
		}
		sensorName := strings.TrimSpace(string(nameBytes))

		// Find temp1_input (main temperature sensor for the device)
		tempBytes, err := os.ReadFile(basePath + "/temp1_input")
		if err != nil {
			continue
		}

		tempStr := strings.TrimSpace(string(tempBytes))
		tempMilliC, err := strconv.ParseFloat(tempStr, 64)
		if err != nil {
			continue
		}

		temp := tempMilliC / 1000.0 // Convert millidegrees to degrees

		// Categorize by sensor type
		switch {
		case sensorName == "k10temp" || sensorName == "coretemp" || strings.Contains(sensorName, "cpu"):
			// AMD k10temp or Intel coretemp = CPU temperature
			if temps.CPUTemp == 0 || temp > temps.CPUTemp {
				temps.CPUTemp = temp
			}
		case sensorName == "amdgpu" || sensorName == "nouveau" || strings.Contains(sensorName, "gpu"):
			// GPU temperature
			if temps.GPUTemp == 0 || temp > temps.GPUTemp {
				temps.GPUTemp = temp
			}
		case strings.Contains(sensorName, "spd") || strings.Contains(sensorName, "ddr"):
			// Memory temperature sensors (DDR5 SPD temperature sensors)
			if temps.MemoryTemp == 0 || temp > temps.MemoryTemp {
				temps.MemoryTemp = temp
			}
		}
	}

	return temps, nil
}

// GetTopProcesses retrieves top processes by CPU and memory usage.
func GetTopProcesses(limit int) ([]ProcessInfo, error) {
	procs, err := process.Processes()
	if err != nil {
		return nil, err
	}

	var processList []ProcessInfo

	for _, p := range procs {
		name, err := p.Name()
		if err != nil {
			continue
		}

		cpuPercent, err := p.CPUPercent()
		if err != nil {
			cpuPercent = 0
		}

		memPercent, err := p.MemoryPercent()
		if err != nil {
			memPercent = 0
		}

		memInfo, err := p.MemoryInfo()
		memBytes := uint64(0)
		if err == nil && memInfo != nil {
			memBytes = memInfo.RSS
		}

		processList = append(processList, ProcessInfo{
			PID:         p.Pid,
			Name:        name,
			CPUPercent:  cpuPercent,
			MemPercent:  memPercent,
			MemoryBytes: memBytes,
		})
	}

	// Sort by CPU usage (descending)
	sort.Slice(processList, func(i, j int) bool {
		return processList[i].CPUPercent > processList[j].CPUPercent
	})

	// Limit results
	if len(processList) > limit {
		processList = processList[:limit]
	}

	return processList, nil
}
