//! Capture mode enumeration

/// Capture mode for screenshots and video recording
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum CaptureMode {
    ImageArea,
    ImageWindow,
    ImageScreen,
    VideoArea,
    VideoWindow,
    VideoScreen,
    /// Synthetic — only produced by the mode-selector UI when a recording is in progress
    #[value(skip)]
    StopRecording,
}

impl CaptureMode {
    /// Returns true if this is a video recording mode.
    pub fn is_video(&self) -> bool {
        matches!(
            self,
            Self::VideoArea | Self::VideoWindow | Self::VideoScreen
        )
    }
}
