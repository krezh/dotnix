package main

import (
	"fmt"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"

	"github.com/spf13/cobra"

	"recshot/internal/capture"
	"recshot/internal/config"
	"recshot/internal/notify"
	"recshot/internal/process"
	"recshot/internal/upload"
)

func main() {
	cfg := config.New()

	cmd := &cobra.Command{
		Use:   "recshot",
		Short: "Screenshot and recording tool with optional Zipline upload",
		Run: func(cmd *cobra.Command, args []string) {
			run(cfg)
		},
	}

	// Configure flags
	cmd.Flags().StringVarP(&cfg.ZiplineURL, "url", "u", cfg.ZiplineURL, "Zipline URL")
	cmd.Flags().StringVarP(&cfg.TokenFile, "token", "t", cfg.TokenFile, "Token file path")
	cmd.Flags().StringVarP(&cfg.Mode, "mode", "m", cfg.Mode, "Mode: image-area, image-window, image-full, video-area, video-window, video-full")
	cmd.Flags().StringVarP(&cfg.SavePath, "path", "p", cfg.SavePath, "Save path")
	cmd.Flags().BoolVarP(&cfg.UseOriginalName, "original-name", "o", cfg.UseOriginalName, "Use original filename")
	cmd.Flags().BoolVar(&cfg.Zipline, "zipline", cfg.Zipline, "Upload to Zipline")
	cmd.Flags().BoolVar(&cfg.Status, "status", cfg.Status, "Show recording status")

	// Custom validation since mode is conditionally required
	cmd.PreRunE = func(cmd *cobra.Command, args []string) error {
		if !cfg.Status && cfg.Mode == "" {
			return fmt.Errorf("mode is required when not using status")
		}
		return nil
	}

	if err := cmd.Execute(); err != nil {
		log.Fatal(err)
	}
}

func run(cfg *config.Config) {
	// Handle status check
	if cfg.Status {
		isRecording, outputFile := getRecordingStatus()
		if isRecording {
			fmt.Printf("🔴 Recording in progress\n")
			if outputFile != "" {
				fmt.Printf("📁 Output: %s\n", outputFile)
			}
			fmt.Println("Run recshot again to stop")
		} else {
			fmt.Println("⏹️ No recording active")
		}
		return
	}

	// Validate configuration
	if err := cfg.Validate(); err != nil {
		log.Fatal(err)
	}

	// Check dependencies
	if err := checkDependencies(cfg.Dependencies); err != nil {
		log.Fatal(err)
	}

	// Create save directory
	if err := os.MkdirAll(cfg.SavePath, 0755); err != nil {
		log.Fatalf("Failed to create save directory: %v", err)
	}

	// Initialize components
	notifier := notify.New()
	procMgr := process.New()
	capturer := capture.New(cfg, notifier, procMgr)

	// Handle video recording check
	if cfg.IsVideoMode() {
		if outputFile, wasRecording, err := capturer.CheckRecording(); err != nil {
			log.Fatal(err)
		} else if wasRecording {
			if outputFile != "" {
				fmt.Printf("Recording saved to: %s\n", outputFile)
				if cfg.Zipline {
					handleUpload(cfg, notifier, outputFile)
				}
			}
			os.Exit(0)
		}
	}

	// Generate filename
	filename := filepath.Join(cfg.SavePath, time.Now().Format("2006-01-02_15-04-05")+cfg.GetFileExtension())

	// Capture based on mode
	if cfg.IsImageMode() {
		if err := capturer.TakeScreenshot(filename); err != nil {
			log.Fatal(err)
		}
		fmt.Printf("Screenshot saved: %s\n", filename)

		if cfg.Zipline {
			handleUpload(cfg, notifier, filename)
		}
	} else if cfg.IsVideoMode() {
		if err := capturer.StartRecording(filename); err != nil {
			log.Fatal(err)
		}
	}
}

func handleUpload(cfg *config.Config, notifier *notify.Notifier, filename string) {
	uploader := upload.NewZiplineUploader(cfg, notifier)
	url, err := uploader.Upload(filename)
	if err != nil {
		log.Printf("Upload failed: %v", err)
		return
	}
	fmt.Printf("Uploaded: %s\n", url)
}

func checkDependencies(deps []string) error {
	for _, dep := range deps {
		if _, err := exec.LookPath(dep); err != nil {
			return fmt.Errorf("%s not found", dep)
		}
	}
	return nil
}

// getRecordingStatus returns recording status and output file
func getRecordingStatus() (bool, string) {
	procMgr := process.New()
	pids, err := procMgr.FindByName("wl-screenrec")
	if err != nil || len(pids) == 0 {
		return false, ""
	}

	// Find output file from first process
	for _, pid := range pids {
		if cmdline, err := procMgr.ReadCmdline(pid); err == nil {
			args := strings.Fields(cmdline)
			for i, arg := range args {
				if arg == "-f" && i+1 < len(args) {
					return true, args[i+1]
				}
			}
		}
	}
	return true, ""
}
