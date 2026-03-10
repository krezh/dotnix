package main

import (
	"kopia-manager/cmd"

	"charm.land/log/v2"
)

func main() {
	if err := cmd.Execute(); err != nil {
		log.Fatal("Command failed", "error", err)
	}
}
