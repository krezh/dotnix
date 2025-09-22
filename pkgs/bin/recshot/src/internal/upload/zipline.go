package upload

import (
	"encoding/json"
	"fmt"
	"io"
	"mime/multipart"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"time"

	"recshot/internal/config"
	"recshot/internal/notify"
)

// ZiplineUploader handles uploads to Zipline
type ZiplineUploader struct {
	config   *config.Config
	notifier *notify.Notifier
	client   *http.Client
	token    string
}

// ZiplineResponse represents the response from Zipline API
type ZiplineResponse struct {
	Files []struct {
		URL string `json:"url"`
	} `json:"files"`
}

// NewZiplineUploader creates a new Zipline uploader
func NewZiplineUploader(cfg *config.Config, notifier *notify.Notifier) *ZiplineUploader {
	// Read token once during initialization
	token, err := os.ReadFile(cfg.TokenFile)
	if err != nil {
		// Return uploader anyway, error will be handled during upload
		return &ZiplineUploader{
			config:   cfg,
			notifier: notifier,
			client: &http.Client{
				Timeout: 30 * time.Second,
			},
		}
	}

	return &ZiplineUploader{
		config:   cfg,
		notifier: notifier,
		token:    strings.TrimSpace(string(token)),
		client: &http.Client{
			Timeout: 30 * time.Second,
		},
	}
}

// Upload uploads a file to Zipline and returns the URL
func (z *ZiplineUploader) Upload(filename string) (string, error) {
	if z.token == "" {
		z.notifier.SendError("Upload failed - no token")
		return "", fmt.Errorf("token not available")
	}

	if z.config.ZiplineURL == "" {
		z.notifier.SendError("Upload failed - no URL")
		return "", fmt.Errorf("Zipline URL not configured")
	}

	file, err := os.Open(filename)
	if err != nil {
		z.notifier.SendError("Upload failed")
		return "", err
	}
	defer file.Close()

	// Create multipart form with streaming
	pipeReader, pipeWriter := io.Pipe()
	writer := multipart.NewWriter(pipeWriter)

	go func() {
		defer pipeWriter.Close()
		defer writer.Close()

		fileWriter, err := writer.CreateFormFile("file", filepath.Base(filename))
		if err != nil {
			pipeWriter.CloseWithError(err)
			return
		}

		io.Copy(fileWriter, file)
	}()

	req, err := http.NewRequest("POST", z.config.ZiplineURL+"/api/upload", pipeReader)
	if err != nil {
		return "", err
	}

	req.Header.Set("Authorization", z.token)
	req.Header.Set("Content-Type", writer.FormDataContentType())
	req.Header.Set("x-zipline-original-name", strconv.FormatBool(z.config.UseOriginalName))

	resp, err := z.client.Do(req)
	if err != nil {
		z.notifier.SendError("Upload failed")
		return "", err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		z.notifier.SendError("Upload failed - server error")
		return "", fmt.Errorf("upload failed with status %d", resp.StatusCode)
	}

	var response ZiplineResponse
	if err := json.NewDecoder(resp.Body).Decode(&response); err != nil || len(response.Files) == 0 {
		z.notifier.SendError("Upload failed - invalid response")
		return "", fmt.Errorf("invalid response")
	}

	url := response.Files[0].URL
	go z.copyToClipboard(url)
	z.notifier.SendSuccessWithAction("Upload successful", url)
	return url, nil
}

// copyToClipboard copies the given text to the system clipboard
func (z *ZiplineUploader) copyToClipboard(text string) {
	cmd := exec.Command("wl-copy")
	cmd.Stdin = strings.NewReader(text)
	cmd.Run() // Ignore errors since this is non-critical
}
