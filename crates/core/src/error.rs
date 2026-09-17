use thiserror::Error;

/// Errors that can occur in audio-bridge core operations
#[derive(Error, Debug)]
pub enum AudioBridgeError {
    #[error("Opus codec error: {0}")]
    OpusError(String),

    #[error("RTP packet error: {0}")]
    RtpError(String),

    #[error("RTP packet validation failed: {0}")]
    PacketValidation(String),

    #[error("Buffer error: {0}")]
    BufferError(String),

    #[error("Invalid audio format: {0}")]
    InvalidFormat(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, AudioBridgeError>;
