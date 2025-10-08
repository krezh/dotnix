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
			client:   newHTTPClient(),
		}
	}

	return &ZiplineUploader{
		config:   cfg,
		notifier: notifier,
		token:    strings.TrimSpace(string(token)),
		client:   newHTTPClient(),
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

	// Create multipart form in memory for retry safety
	body, contentType, err := z.createMultipartBody(filename)
	if err != nil {
		z.notifier.SendError("Upload failed")
		return "", err
	}

	req, err := http.NewRequest("POST", z.config.ZiplineURL+"/api/upload", nil)
	if err != nil {
		return "", err
	}

	// Set GetBody for retry safety with HTTP/2
	req.GetBody = func() (io.ReadCloser, error) {
		return io.NopCloser(strings.NewReader(body)), nil
	}
	req.Body = io.NopCloser(strings.NewReader(body))
	req.ContentLength = int64(len(body))

	req.Header.Set("Authorization", z.token)
	req.Header.Set("Content-Type", contentType)
	req.Header.Set("x-zipline-original-name", strconv.FormatBool(z.config.UseOriginalName))
	req.Header.Set("User-Agent", "recshot/1.0")

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
	fmt.Printf("Uploaded: %s\n", url)
	go z.copyToClipboard(url)
	z.notifier.SendSuccessWithAction("Upload successful", url)
	return url, nil
}

// createMultipartBody creates a multipart form body in memory
func (z *ZiplineUploader) createMultipartBody(filename string) (string, string, error) {
	file, err := os.Open(filename)
	if err != nil {
		return "", "", err
	}
	defer file.Close()

	var body strings.Builder
	writer := multipart.NewWriter(&body)

	fileWriter, err := writer.CreateFormFile("file", filepath.Base(filename))
	if err != nil {
		return "", "", err
	}

	// Use buffered copying for better performance
	const bufSize = 32 * 1024 // 32KB buffer
	_, err = io.CopyBuffer(fileWriter, file, make([]byte, bufSize))
	if err != nil {
		return "", "", err
	}

	contentType := writer.FormDataContentType()
	writer.Close()

	return body.String(), contentType, nil
}

// newHTTPClient creates an HTTP client with connection pooling and HTTP/2 fixes
func newHTTPClient() *http.Client {
	transport := &http.Transport{
		MaxIdleConns:        10,
		MaxIdleConnsPerHost: 5,
		IdleConnTimeout:     30 * time.Second,
		DisableCompression:  false, // Enable compression for uploads
		// Force HTTP/1.1 to avoid HTTP/2 retry issues with large uploads
		ForceAttemptHTTP2: false,
	}

	return &http.Client{
		Timeout:   30 * time.Second,
		Transport: transport,
	}
}

// copyToClipboard copies the given text to the system clipboard
func (z *ZiplineUploader) copyToClipboard(text string) {
	cmd := exec.Command("wl-copy")
	cmd.Stdin = strings.NewReader(text)
	cmd.Run() // Ignore errors since this is non-critical
}
