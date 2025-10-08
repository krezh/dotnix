package main

import (
	"kopia-manager/cmd"

	"github.com/charmbracelet/log"
)

func main() {
	if err := cmd.Execute(); err != nil {
		log.Fatal("Command failed", "error", err)
	}
}
