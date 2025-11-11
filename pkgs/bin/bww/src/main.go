package main

import (
	"flag"
	"fmt"
	"log"
	"os"

	"bww/internal/bw"
	"bww/internal/ui"
)

func main() {
	loginFlag := flag.Bool("login", false, "Login to Bitwarden (logout first if needed)")
	lockFlag := flag.Bool("lock", false, "Lock the vault")
	syncFlag := flag.Bool("sync", false, "Sync vault with server")
	unlockFlag := flag.Bool("unlock", false, "Unlock the vault")
	helpFlag := flag.Bool("help", false, "Show help message")
	flag.BoolVar(helpFlag, "h", false, "Show help message")

	flag.Usage = func() {
		fmt.Fprintf(os.Stderr, "Bitwarden Walker - A Bitwarden menu interface using Walker\n\n")
		fmt.Fprintf(os.Stderr, "Usage: bww [OPTION]\n\n")
		fmt.Fprintf(os.Stderr, "Options:\n")
		fmt.Fprintf(os.Stderr, "  --login     Login to Bitwarden (logout first if needed)\n")
		fmt.Fprintf(os.Stderr, "  --unlock    Unlock the vault\n")
		fmt.Fprintf(os.Stderr, "  --lock      Lock the vault\n")
		fmt.Fprintf(os.Stderr, "  --sync      Sync vault with server\n")
		fmt.Fprintf(os.Stderr, "  --help      Show this help message\n\n")
		fmt.Fprintf(os.Stderr, "When run without options, opens the item search interface.\n")
	}

	flag.Parse()

	if *helpFlag {
		flag.Usage()
		return
	}

	client := bw.NewClient()

	if *loginFlag {
		if err := client.Logout(); err != nil {
			log.Printf("Warning: logout failed: %v", err)
		}
		if err := ui.HandleLogin(client); err != nil {
			log.Fatalf("Login failed: %v", err)
		}
		return
	}

	if *lockFlag {
		if err := client.Lock(); err != nil {
			log.Fatalf("Lock failed: %v", err)
		}
		ui.Notify("Vault locked")
		return
	}

	if *syncFlag {
		if err := ui.HandleSync(client); err != nil {
			log.Fatalf("Sync failed: %v", err)
		}
		return
	}

	if *unlockFlag {
		if err := ui.HandleUnlock(client); err != nil {
			log.Fatalf("Unlock failed: %v", err)
		}
		return
	}

	// Default: open item search
	if err := ui.HandleItemSearch(client); err != nil {
		log.Fatalf("Error: %v", err)
	}
}
