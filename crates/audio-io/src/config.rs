/// Opus encoding bitrate options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bitrate {
    Kbps64 = 64000,
    Kbps96 = 96000,
    Kbps128 = 128000,
    Kbps192 = 192000,
}

impl Bitrate {
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

/// Audio sender configuration
#[derive(Debug, Clone)]
pub struct SenderConfig {
    /// Opus encoding bitrate
    pub bitrate: Bitrate,

    /// Audio capture device (None = default device)
    pub capture_device: Option<String>,

    /// RTP SSRC identifier
    pub ssrc: u32,
}

impl Default for SenderConfig {
    fn default() -> Self {
        Self {
            bitrate: Bitrate::Kbps128,
            capture_device: None,
            ssrc: rand::random(),
        }
    }
}

/// Audio receiver configuration
#[derive(Debug, Clone)]
pub struct ReceiverConfig {
    /// Opus decoding bitrate (must match sender)
    pub bitrate: Bitrate,

    /// Audio playback device (None = default device)
    pub playback_device: Option<String>,

    /// Jitter buffer capacity (number of packets)
    pub jitter_buffer_size: usize,
}

impl Default for ReceiverConfig {
    fn default() -> Self {
        Self {
            bitrate: Bitrate::Kbps128,
            playback_device: None,
            jitter_buffer_size: 50,
        }
    }
}

/// Audio format constants (fixed for Phase 1c)
pub mod format {
    pub const SAMPLE_RATE: u32 = 48000;
    pub const CHANNELS: u16 = 2;
    pub const FRAME_DURATION_MS: u32 = 20;
    pub const SAMPLES_PER_CHANNEL: usize = 960; // 48000 * 0.02
    pub const FRAME_SIZE: usize = 1920; // 960 * 2 channels
}
