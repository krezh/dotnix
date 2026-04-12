package system

import (
	"bufio"
	"fmt"
	"io"
	"os/exec"
	"sync"
)

// runWithSudo starts a command that requires sudo, sends the password via stdin,
// and waits for completion. The cmd must already have its arguments set.
func runWithSudo(cmd *exec.Cmd, sudoPassword string) error {
	stdin, err := cmd.StdinPipe()
	if err != nil {
		return fmt.Errorf("failed to create stdin pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("failed to start command: %w", err)
	}

	if sudoPassword != "" {
		if _, err = stdin.Write([]byte(sudoPassword + "\n")); err != nil {
			return fmt.Errorf("failed to write password: %w", err)
		}
	}
	stdin.Close()

	if err := cmd.Wait(); err != nil {
		return fmt.Errorf("command failed: %w", err)
	}

	return nil
}

// streamCommandOutput sets up stdout/stderr pipes, starts the command, streams
// all output lines to progressChan under the given step label, and waits for
// the command to finish. The progressChan is not closed by this function.
func streamCommandOutput(cmd *exec.Cmd, progressChan chan<- UpdateProgress, step string) error {
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return fmt.Errorf("failed to create stdout pipe: %w", err)
	}

	stderr, err := cmd.StderrPipe()
	if err != nil {
		return fmt.Errorf("failed to create stderr pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("failed to start command: %w", err)
	}

	var wg sync.WaitGroup
	wg.Add(2)
	go streamLines(stdout, progressChan, step, &wg)
	go streamLines(stderr, progressChan, step, &wg)
	wg.Wait()

	if err := cmd.Wait(); err != nil {
		return err
	}

	return nil
}

// streamCommandOutputWithSudo sets up stdin for sudo password, streams stdout and
// stderr to progressChan, and waits for the command to finish.
func streamCommandOutputWithSudo(cmd *exec.Cmd, sudoPassword string, progressChan chan<- UpdateProgress, step string) error {
	stdin, err := cmd.StdinPipe()
	if err != nil {
		return fmt.Errorf("failed to create stdin pipe: %w", err)
	}

	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return fmt.Errorf("failed to create stdout pipe: %w", err)
	}

	stderr, err := cmd.StderrPipe()
	if err != nil {
		return fmt.Errorf("failed to create stderr pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("failed to start command: %w", err)
	}

	if sudoPassword != "" {
		if _, err = stdin.Write([]byte(sudoPassword + "\n")); err != nil {
			return fmt.Errorf("failed to write password: %w", err)
		}
	}
	stdin.Close()

	var wg sync.WaitGroup
	wg.Add(2)
	go streamLines(stdout, progressChan, step, &wg)
	go streamLines(stderr, progressChan, step, &wg)
	wg.Wait()

	if err := cmd.Wait(); err != nil {
		return err
	}

	return nil
}

// streamLines reads lines from reader and sends them as UpdateProgress messages.
func streamLines(reader io.Reader, progressChan chan<- UpdateProgress, step string, wg *sync.WaitGroup) {
	defer wg.Done()

	scanner := bufio.NewScanner(reader)
	for scanner.Scan() {
		progressChan <- UpdateProgress{
			Step:    step,
			Message: scanner.Text(),
		}
	}
}
