//! Capture mode enumeration and parsing

use anyhow::Result;

/// Capture mode for screenshots and video recording
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    ImageArea,
    ImageWindow,
    ImageScreen,
    VideoArea,
    VideoWindow,
    VideoScreen,
    /// Synthetic — only produced by the mode-selector UI when a recording is in progress
    StopRecording,
}

impl CaptureMode {
    /// Parses a mode string into a CaptureMode.
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "image-area" => Ok(Self::ImageArea),
            "image-window" => Ok(Self::ImageWindow),
            "image-screen" => Ok(Self::ImageScreen),
            "video-area" => Ok(Self::VideoArea),
            "video-window" => Ok(Self::VideoWindow),
            "video-screen" => Ok(Self::VideoScreen),
            _ => Err(anyhow::anyhow!(
                "Invalid mode: '{}'. Valid modes: image-area, image-window, image-screen, video-area, video-window, video-screen",
                s
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ImageArea     => "image-area",
            Self::ImageWindow   => "image-window",
            Self::ImageScreen   => "image-screen",
            Self::VideoArea     => "video-area",
            Self::VideoWindow   => "video-window",
            Self::VideoScreen   => "video-screen",
            Self::StopRecording => "stop-recording",
        }
    }

    /// Returns true if this is a video recording mode.
    pub fn is_video(&self) -> bool {
        matches!(
            self,
            Self::VideoArea | Self::VideoWindow | Self::VideoScreen
        )
    }
}
