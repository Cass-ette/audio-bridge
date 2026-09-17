use crate::{traits::AudioCapture, AudioIoError, Result, SenderConfig};
use audio_bridge_core::codec::{AudioFormat, OpusEncoder};
use audio_bridge_core::rtp::RtpPacket;
use audio_bridge_core::transport::RtpSender;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Audio sender statistics
#[derive(Debug, Clone, Default)]
pub struct SenderStats {
    pub packets_sent: u64,
    pub bytes_sent: u64,
    pub frames_captured: u64,
    pub encoding_errors: u64,
}

/// Audio sender pipeline
///
/// Captures audio → Encodes → Sends via RTP
pub struct AudioSender {
    capture: Box<dyn AudioCapture>,
    encoder: OpusEncoder,
    rtp_sender: RtpSender,
    config: SenderConfig,
    running: Arc<AtomicBool>,
    stats: Arc<Mutex<SenderStats>>,
    sequence_number: u16,
}

impl AudioSender {
    /// Get a handle to control the running state
    pub fn running_handle(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }
}
