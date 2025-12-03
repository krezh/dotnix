package ui

import (
	"bufio"
	"fmt"
	"os/exec"
	"regexp"
	"strings"

	"bww/internal/bw"
	"bww/internal/clipboard"
)

func Notify(message string) {
	exec.Command("notify-send", "--transient", "-t", "2000", "-u", "normal", "-a", "Bitwarden", message).Run()
}

func GetPassphrase(prompt string) (string, error) {
	// Try different pinentry variants
	pinentries := []string{"pinentry-gnome3", "pinentry-qt", "pinentry-gtk-2", "pinentry-curses", "pinentry"}

	var pinentryCmd string
	for _, p := range pinentries {
		if _, err := exec.LookPath(p); err == nil {
			pinentryCmd = p
			break
		}
	}

	if pinentryCmd == "" {
		return "", fmt.Errorf("pinentry not found")
	}

	cmd := exec.Command(pinentryCmd)
	stdin, err := cmd.StdinPipe()
	if err != nil {
		return "", err
	}
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return "", err
	}

	if err := cmd.Start(); err != nil {
		return "", err
	}

	// Send pinentry commands
	fmt.Fprintf(stdin, "SETDESC Enter %s\n", prompt)
	fmt.Fprintf(stdin, "GETPIN\n")
	stdin.Close()

	// Parse output
	scanner := bufio.NewScanner(stdout)
	var password string
	for scanner.Scan() {
		line := scanner.Text()
		if after, found := strings.CutPrefix(line, "D "); found {
			password = after
			break
		}
	}

	cmd.Wait()
	return password, nil
}

func WalkerSelect(prompt string, items []string) (string, error) {
	cmd := exec.Command("walker", "--dmenu", "-p", prompt)
	stdin, err := cmd.StdinPipe()
	if err != nil {
		return "", err
	}

	go func() {
		defer stdin.Close()
		for _, item := range items {
			fmt.Fprintln(stdin, item)
		}
	}()

	output, err := cmd.Output()
	if err != nil {
		return "", err
	}

	return strings.TrimSpace(string(output)), nil
}

func WalkerInput(prompt string) (string, error) {
	cmd := exec.Command("walker", "--dmenu", "-p", prompt)
	stdin, _ := cmd.StdinPipe()
	stdin.Close()

	output, err := cmd.Output()
	if err != nil {
		return "", err
	}

	return strings.TrimSpace(string(output)), nil
}

func HandleLogin(client *bw.Client) error {
	email, err := WalkerInput("Bitwarden Email")
	if err != nil || email == "" {
		Notify("Login cancelled")
		return fmt.Errorf("login cancelled")
	}

	password, err := GetPassphrase("Master Password")
	if err != nil || password == "" {
		Notify("Password entry cancelled")
		return fmt.Errorf("password entry cancelled")
	}

	Notify("Logging in to Bitwarden...")

	err = client.Login(email, password, "", "")
	if err != nil {
		// Check if 2FA is required
		if strings.Contains(err.Error(), "Two-step login code") {
			method, err := WalkerSelect("Select 2FA Method", []string{"Authenticator App", "Email", "Yubikey"})
			if err != nil || method == "" {
				Notify("2FA cancelled")
				return fmt.Errorf("2FA cancelled")
			}

			methodMap := map[string]string{
				"Authenticator App": "0",
				"Email":             "1",
				"Yubikey":           "3",
			}

			code, err := GetPassphrase("2FA Code")
			if err != nil || code == "" {
				Notify("2FA cancelled")
				return fmt.Errorf("2FA cancelled")
			}

			err = client.Login(email, password, methodMap[method], code)
			if err != nil {
				Notify("Login failed: Invalid 2FA code")
				return err
			}

			Notify("Login successful with 2FA!")
			return nil
		}

		Notify(fmt.Sprintf("Login failed: %v", err))
		return err
	}

	Notify("Login successful!")
	return nil
}

func HandleUnlock(client *bw.Client) error {
	password, err := GetPassphrase("Master Password")
	if err != nil || password == "" {
		Notify("Password entry cancelled")
		return fmt.Errorf("password entry cancelled")
	}

	if err := client.Unlock(password); err != nil {
		// If unlock fails, might need to login
		if strings.Contains(err.Error(), "not logged in") || strings.Contains(err.Error(), "unauthenticated") {
			return HandleLogin(client)
		}
		Notify("Failed to unlock vault")
		return err
	}

	Notify("Vault unlocked successfully")
	return nil
}

func HandleSync(client *bw.Client) error {
	Notify("Syncing vault...")
	err := client.Sync()
	if err != nil {
		// If sync fails due to authentication, unlock and try again
		if strings.Contains(err.Error(), "not logged in") || strings.Contains(err.Error(), "unauthenticated") {
			if err := HandleUnlock(client); err != nil {
				return err
			}
			err = client.Sync()
		}
		if err != nil {
			Notify("Failed to sync vault")
			return err
		}
	}

	Notify("Vault synced successfully")
	return nil
}

func HandleItemSearch(client *bw.Client) error {
	for {
		// Try to get items with existing session (if any)
		items, err := client.GetItems()
		if err != nil {
			// If failed, likely need to unlock
			if err := HandleUnlock(client); err != nil {
				return err
			}
			// Try again after unlock
			items, err = client.GetItems()
			if err != nil {
				Notify("Failed to fetch items")
				return err
			}
		}

		// Filter login items only
		var loginItems []bw.Item
		for _, item := range items {
			if item.Type == 1 && item.Login != nil {
				loginItems = append(loginItems, item)
			}
		}

		if len(loginItems) == 0 {
			Notify("No items found in vault")
			return nil
		}

		// Format items for display
		itemMap := make(map[string]bw.Item)
		var displayItems []string

		// Add refresh option at the top
		refreshOption := "🔄 Refresh Items"
		displayItems = append(displayItems, refreshOption)

		for _, item := range loginItems {
			display := item.Name

			if item.Login.Username != "" {
				display += " (" + item.Login.Username + ")"
			}

			if len(item.Login.URIs) > 0 && item.Login.URIs[0].URI != "" {
				// Extract domain from URI
				re := regexp.MustCompile(`([a-zA-Z0-9.-]+\.[a-zA-Z]{2,})`)
				if matches := re.FindStringSubmatch(item.Login.URIs[0].URI); len(matches) > 0 {
					display += " [" + matches[0] + "]"
				}
			}

			displayItems = append(displayItems, display)
			itemMap[display] = item
		}

		selected, err := WalkerSelect("Bitwarden Items", displayItems)
		if err != nil || selected == "" {
			return nil
		}

		// Handle refresh option
		if selected == refreshOption {
			Notify("Syncing vault...")
			if err := client.Sync(); err != nil {
				Notify("Failed to sync vault")
			} else {
				Notify("Vault synced successfully")
			}
			continue
		}

		item, ok := itemMap[selected]
		if !ok {
			return nil
		}

		return HandleItemAction(client, item)
	}
}

func HandleItemAction(client *bw.Client, item bw.Item) error {
	actions := []string{}

	if item.Login.Password != "" {
		actions = append(actions, "Copy Password")
	}
	if item.Login.Username != "" {
		actions = append(actions, "Copy Username")
	}
	if item.Login.TOTP != nil && *item.Login.TOTP != "" {
		actions = append(actions, "Copy TOTP")
	}
	actions = append(actions, "View Details", "Copy Item ID")

	action, err := WalkerSelect(fmt.Sprintf("Action for: %s", item.Name), actions)
	if err != nil || action == "" {
		return nil
	}

	switch action {
	case "Copy Password":
		clipboard.Copy(item.Login.Password)
		Notify("Copied password to clipboard")

	case "Copy Username":
		clipboard.Copy(item.Login.Username)
		Notify("Copied username to clipboard")

	case "Copy TOTP":
		totp, err := client.GetTOTP(item.ID)
		if err != nil {
			Notify("Failed to get TOTP")
			return err
		}
		clipboard.Copy(totp)
		Notify("Copied TOTP code to clipboard")

	case "View Details":
		details := []string{
			"Name: " + item.Name,
		}
		if item.Login.Username != "" {
			details = append(details, "Username: "+item.Login.Username)
		}
		if len(item.Login.URIs) > 0 {
			details = append(details, "URI: "+item.Login.URIs[0].URI)
		}
		if item.Notes != nil && *item.Notes != "" {
			details = append(details, "", "Notes:", *item.Notes)
		}
		WalkerSelect("Item Details", details)

	case "Copy Item ID":
		clipboard.Copy(item.ID)
		Notify("Copied item ID to clipboard")
	}

	return nil
}
