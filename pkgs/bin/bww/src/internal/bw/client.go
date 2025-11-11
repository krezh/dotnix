package bw

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"

	"github.com/charmbracelet/log"
)

type Client struct {
	cacheDir  string
	cacheFile string
}

type Item struct {
	ID             string        `json:"id"`
	Name           string        `json:"name"`
	Type           int           `json:"type"`
	Login          *LoginInfo    `json:"login,omitempty"`
	FolderID       *string       `json:"folderId"`
	OrganizationID *string       `json:"organizationId"`
	Notes          *string       `json:"notes"`
	Fields         []CustomField `json:"fields,omitempty"`
}

type LoginInfo struct {
	Username string  `json:"username,omitempty"`
	Password string  `json:"password,omitempty"`
	TOTP     *string `json:"totp,omitempty"`
	URIs     []URI   `json:"uris,omitempty"`
}

type URI struct {
	URI   string `json:"uri"`
	Match *int   `json:"match,omitempty"`
}

type CustomField struct {
	Name  string `json:"name"`
	Value string `json:"value"`
	Type  int    `json:"type"`
}

func NewClient() *Client {
	cacheDir := os.Getenv("XDG_CACHE_HOME")
	if cacheDir == "" {
		cacheDir = filepath.Join(os.Getenv("HOME"), ".cache")
	}
	cacheDir = filepath.Join(cacheDir, "bww")

	if err := os.MkdirAll(cacheDir, 0755); err != nil {
		fmt.Fprintf(os.Stderr, "Warning: could not create cache dir: %v\n", err)
	}

	return &Client{
		cacheDir:  cacheDir,
		cacheFile: filepath.Join(cacheDir, "items.json"),
	}
}

func (c *Client) Login(email, password, method, code string) error {
	log.Debug("Logging in", "email", email)
	args := []string{"login", "--raw", email, password}
	if method != "" && code != "" {
		log.Debug("Using 2FA", "method", method)
		args = append(args, "--method", method, "--code", code)
	}

	cmd := exec.Command("bw", args...)
	output, err := cmd.CombinedOutput()
	if err != nil {
		log.Debug("Login failed", "output", string(output))
		return fmt.Errorf("login failed: %s", string(output))
	}

	session := strings.TrimSpace(string(output))
	log.Debug("Login successful", "session_length", len(session))
	return c.saveSession(session)
}

func (c *Client) Unlock(password string) error {
	log.Debug("Unlocking vault")

	cmd := exec.Command("bw", "unlock", "--raw", password)
	output, err := cmd.CombinedOutput()
	if err != nil {
		log.Debug("Unlock failed", "output", string(output))
		return fmt.Errorf("unlock failed: %s", string(output))
	}

	session := strings.TrimSpace(string(output))
	log.Debug("Unlock successful", "session_length", len(session))
	return c.saveSession(session)
}

func (c *Client) Lock() error {
	_ = exec.Command("bw", "lock").Run()
	_ = exec.Command("secret-tool", "clear", "application", "bww", "type", "session").Run()
	_ = os.Remove(c.cacheFile)
	return nil
}

func (c *Client) Logout() error {
	_ = exec.Command("bw", "logout").Run()
	_ = exec.Command("secret-tool", "clear", "application", "bww", "type", "session").Run()
	_ = os.Remove(c.cacheFile)
	return nil
}

func (c *Client) Sync() error {
	session, err := c.getSession()
	if err != nil {
		session = ""
	}

	cmd := exec.Command("bw", "--session", session, "sync")
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("sync failed: %w", err)
	}

	_ = os.Remove(c.cacheFile)
	return nil
}

func (c *Client) GetItems() ([]Item, error) {
	log.Debug("Getting items")
	// Check cache first
	if _, err := os.Stat(c.cacheFile); err == nil {
		data, err := os.ReadFile(c.cacheFile)
		if err == nil {
			var items []Item
			if err := json.Unmarshal(data, &items); err == nil {
				log.Debug("Using cached items", "count", len(items))
				return items, nil
			}
		}
	}

	log.Debug("Fetching items from bw")
	// Fetch from bw
	session, err := c.getSession()
	if err != nil {
		log.Debug("No session, using empty")
		// Try with empty session if vault is unlocked
		session = ""
	}

	cmd := exec.Command("bw", "--session", session, "list", "items")
	output, err := cmd.Output()
	if err != nil {
		log.Debug("Failed to list items", "error", err)
		return nil, fmt.Errorf("failed to list items: %w", err)
	}

	var items []Item
	if err := json.Unmarshal(output, &items); err != nil {
		log.Debug("Failed to parse items", "error", err)
		return nil, fmt.Errorf("failed to parse items: %w", err)
	}

	log.Debug("Got items, caching", "count", len(items))
	// Cache the results
	_ = os.WriteFile(c.cacheFile, output, 0600)

	return items, nil
}

func (c *Client) GetTOTP(itemID string) (string, error) {
	session, err := c.getSession()
	if err != nil {
		session = ""
	}

	cmd := exec.Command("bw", "--session", session, "get", "totp", itemID)
	output, err := cmd.Output()
	if err != nil {
		return "", fmt.Errorf("failed to get TOTP: %w", err)
	}

	return strings.TrimSpace(string(output)), nil
}

func (c *Client) getSession() (string, error) {
	log.Debug("Getting session")
	cmd := exec.Command("secret-tool", "lookup", "application", "bww", "type", "session")
	output, err := cmd.Output()
	if err != nil {
		log.Debug("No session in keyring", "error", err)
		return "", fmt.Errorf("no session found: %w", err)
	}
	if len(output) == 0 {
		log.Debug("Empty session in keyring")
		return "", fmt.Errorf("no session found")
	}
	session := strings.TrimSpace(string(output))
	log.Debug("Got session from keyring", "length", len(session))
	return session, nil
}

func (c *Client) saveSession(session string) error {
	log.Debug("Saving session", "length", len(session))

	cmd := exec.Command("secret-tool", "store", "--label", "Bitwarden Session", "application", "bww", "type", "session")
	stdin, err := cmd.StdinPipe()
	if err != nil {
		log.Debug("Failed to open keyring pipe", "error", err)
		return fmt.Errorf("failed to save session: %w", err)
	}

	if err := cmd.Start(); err != nil {
		log.Debug("Failed to start keyring", "error", err)
		return fmt.Errorf("failed to save session: %w", err)
	}

	stdin.Write([]byte(session))
	stdin.Close()

	if err := cmd.Wait(); err != nil {
		log.Debug("Failed to save to keyring", "error", err)
		return fmt.Errorf("failed to save session: %w", err)
	}

	log.Debug("Saved to keyring")
	return nil
}
