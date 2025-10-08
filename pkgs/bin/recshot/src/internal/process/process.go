package process

import (
	"bufio"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
	"syscall"
)

// Manager handles process operations
type Manager struct{}

// New creates a new process manager
func New() *Manager {
	return &Manager{}
}

// FindByName finds all PIDs with the given name using parallel processing
func (m *Manager) FindByName(name string) ([]int, error) {
	procDirs, err := filepath.Glob("/proc/[0-9]*")
	if err != nil {
		return nil, err
	}

	const numWorkers = 4 // Optimal based on benchmark results
	jobs := make(chan string, len(procDirs))
	results := make(chan int, len(procDirs))

	var wg sync.WaitGroup
	for i := 0; i < numWorkers; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for procDir := range jobs {
				pidStr := filepath.Base(procDir)
				pid, err := strconv.Atoi(pidStr)
				if err != nil {
					continue
				}

				commPath := filepath.Join(procDir, "comm")
				if processName := m.readCommFileBuffered(commPath); processName == name {
					results <- pid
				}
			}
		}()
	}

	go func() {
		defer close(jobs)
		for _, procDir := range procDirs {
			jobs <- procDir
		}
	}()

	go func() {
		wg.Wait()
		close(results)
	}()

	var pids []int
	for pid := range results {
		pids = append(pids, pid)
	}

	return pids, nil
}

// readCommFileBuffered reads process name from comm file efficiently
func (m *Manager) readCommFileBuffered(commPath string) string {
	file, err := os.Open(commPath)
	if err != nil {
		return ""
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	if scanner.Scan() {
		return strings.TrimSpace(scanner.Text())
	}
	return ""
}

// Kill terminates a process with the given signal
func (m *Manager) Kill(pid int, signal syscall.Signal) error {
	return syscall.Kill(pid, signal)
}

// KillByName terminates all processes with the given name
func (m *Manager) KillByName(name string, signal syscall.Signal) error {
	pids, err := m.FindByName(name)
	if err != nil {
		return err
	}
	for _, pid := range pids {
		syscall.Kill(pid, signal)
	}
	return nil
}

// ReadCmdline reads the command line of a process using buffered I/O
func (m *Manager) ReadCmdline(pid int) (string, error) {
	cmdlinePath := filepath.Join("/proc", strconv.Itoa(pid), "cmdline")

	file, err := os.Open(cmdlinePath)
	if err != nil {
		return "", err
	}
	defer file.Close()

	// Use buffered reading for better performance
	scanner := bufio.NewScanner(file)
	scanner.Split(bufio.ScanBytes)

	var result strings.Builder
	for scanner.Scan() {
		b := scanner.Bytes()[0]
		if b == 0 {
			result.WriteByte(' ') // Replace null bytes with spaces
		} else {
			result.WriteByte(b)
		}
	}

	return strings.TrimSpace(result.String()), scanner.Err()
}
