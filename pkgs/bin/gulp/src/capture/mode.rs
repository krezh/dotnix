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

    /// Returns true if this is a video recording mode.
    pub fn is_video(&self) -> bool {
        matches!(
            self,
            Self::VideoArea | Self::VideoWindow | Self::VideoScreen
        )
    }
}
