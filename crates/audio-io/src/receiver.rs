use crate::{traits::AudioPlayback, AudioIoError, ReceiverConfig, Result};
use audio_bridge_core::buffer::JitterBuffer;
use audio_bridge_core::codec::{AudioFormat, OpusDecoder};
use audio_bridge_core::transport::RtpReceiver;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Audio receiver statistics
#[derive(Debug, Clone, Default)]
pub struct ReceiverStats {
    pub packets_received: u64,
    pub packets_decoded: u64,
    pub packets_lost: u64,
    pub bytes_received: u64,
    pub decoding_errors: u64,
}

/// Audio receiver pipeline
///
/// Receives RTP → Jitter Buffer → Decodes → Plays audio
pub struct AudioReceiver {
    rtp_receiver: RtpReceiver,
    jitter_buffer: JitterBuffer,
    decoder: OpusDecoder,
    playback: Box<dyn AudioPlayback>,
    config: ReceiverConfig,
    running: Arc<AtomicBool>,
    stats: Arc<Mutex<ReceiverStats>>,
}

impl AudioReceiver {
    /// Get a handle to control the running state
    pub fn running_handle(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }
}
