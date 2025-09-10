package process

import (
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"syscall"
)

// Manager handles process operations
type Manager struct{}

// New creates a new process manager
func New() *Manager {
	return &Manager{}
}

// FindByName finds all PIDs with the given name
func (m *Manager) FindByName(name string) ([]int, error) {
	var pids []int
	entries, err := os.ReadDir("/proc")
	if err != nil {
		return nil, err
	}

	for _, entry := range entries {
		if pid, err := strconv.Atoi(entry.Name()); err == nil {
			if comm, err := os.ReadFile(filepath.Join("/proc", entry.Name(), "comm")); err == nil {
				if strings.TrimSpace(string(comm)) == name {
					pids = append(pids, pid)
				}
			}
		}
	}
	return pids, nil
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

// ReadCmdline reads the command line of a process
func (m *Manager) ReadCmdline(pid int) (string, error) {
	data, err := os.ReadFile(filepath.Join("/proc", strconv.Itoa(pid), "cmdline"))
	if err != nil {
		return "", err
	}
	return strings.TrimSpace(strings.ReplaceAll(string(data), "\x00", " ")), nil
}
