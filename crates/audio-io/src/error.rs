//! Error types for audio-io crate
//!
//! Errors are categorized by severity:
//! - Fatal: Cannot recover, must stop (device not found, unsupported format)
//! - Temporary: Can retry in Phase 2 (network timeout, buffer overrun/underrun)
//! - Status: Notify upper layer (device disconnected/changed)

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioIoError {
    // Fatal errors - immediate stop
    #[error("Audio device not found: {0}")]
    DeviceNotFound(String),

    #[error("Failed to open audio device: {0}")]
    DeviceOpenFailed(String),

    #[error("Unsupported audio format: {0}")]
    UnsupportedFormat(String),

    // Temporary errors - can retry (Phase 1c: no auto-retry)
    #[error("Network timeout")]
    NetworkTimeout,

    #[error("Audio buffer overrun")]
    BufferOverrun,

    #[error("Audio buffer underrun")]
    BufferUnderrun,

    // Status changes - notify upper layer
    #[error("Audio device disconnected")]
    DeviceDisconnected,

    #[error("Audio device changed to: {0}")]
    DeviceChanged(String),

    // Errors from lower-level modules
    #[error("Codec error: {0}")]
    Codec(#[from] audio_bridge_core::AudioBridgeError),

    #[error("Platform error: {0}")]
    Platform(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("WAV parse error: {0}")]
    WavParse(String),
}

impl AudioIoError {
    /// Check if this is a fatal error (cannot recover)
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            Self::DeviceNotFound(_) | Self::DeviceOpenFailed(_) | Self::UnsupportedFormat(_)
        )
    }

    /// Check if this error should trigger retry (for Phase 2)
    pub fn should_retry(&self) -> bool {
        matches!(
            self,
            Self::NetworkTimeout | Self::BufferOverrun | Self::BufferUnderrun
        )
    }
}

pub type Result<T> = std::result::Result<T, AudioIoError>;
