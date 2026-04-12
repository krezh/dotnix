package handlers

import (
	"log"
	"net/http"

	"github.com/krezh/nixos-webgui/system"
	"github.com/krezh/nixos-webgui/templates"
)

// HandleGitStatus returns an HTML indicator for remote commits available to pull.
func HandleGitStatus(w http.ResponseWriter, r *http.Request) {
	status, err := system.GetGitStatus()
	if err != nil {
		log.Printf("Error getting git status: %v", err)
		renderComponent(w, r, templates.GitStatusIndicator(nil))
		return
	}
	renderComponent(w, r, templates.GitStatusIndicator(status))
}

// HandleGitStatusDot returns a small nav dot indicator for remote commits available to pull.
func HandleGitStatusDot(w http.ResponseWriter, r *http.Request) {
	status, err := system.GetGitStatus()
	if err != nil {
		log.Printf("Error getting git status: %v", err)
		renderComponent(w, r, templates.GitStatusDot(0))
		return
	}
	renderComponent(w, r, templates.GitStatusDot(status.Behind))
}

// HandleGitStatusRefresh forces a cache refresh and returns a fresh indicator.
func HandleGitStatusRefresh(w http.ResponseWriter, r *http.Request) {
	status, err := system.RefreshGitStatus()
	if err != nil {
		log.Printf("Error refreshing git status: %v", err)
		renderComponent(w, r, templates.GitStatusIndicator(nil))
		return
	}
	renderComponent(w, r, templates.GitStatusIndicator(status))
}
