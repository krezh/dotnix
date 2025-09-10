package main

func main() {
	// Initialize commands
	initCommands()

	if err := rootCmd.Execute(); err != nil {
		logger.Fatal("Command failed", "error", err)
	}
}
