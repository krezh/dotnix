package clipboard

import (
	"os"
	"os/exec"
	"strconv"
	"time"
)

func Copy(text string) {
	cmd := exec.Command("wl-copy")
	stdin, err := cmd.StdinPipe()
	if err != nil {
		return
	}

	if err := cmd.Start(); err != nil {
		return
	}

	stdin.Write([]byte(text))
	stdin.Close()
	cmd.Wait()

	// Auto-clear after timeout
	go func() {
		timeout := 30
		if t := os.Getenv("BW_CLIP_CLEAR_TIME"); t != "" {
			if parsed, err := strconv.Atoi(t); err == nil {
				timeout = parsed
			}
		}

		time.Sleep(time.Duration(timeout) * time.Second)

		// Only clear if clipboard still contains our text
		getCmd := exec.Command("wl-paste")
		output, err := getCmd.Output()
		if err == nil && string(output) == text {
			exec.Command("wl-copy").Run()
		}
	}()
}
