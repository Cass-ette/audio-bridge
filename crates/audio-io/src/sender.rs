use crate::{traits::AudioCapture, AudioIoError, Result, SenderConfig};
use audio_bridge_core::codec::{AudioFormat, OpusEncoder};
use audio_bridge_core::transport::RtpSender;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
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
    /// Create a new AudioSender with a provided capture device
    pub async fn with_capture(
        capture: Box<dyn AudioCapture>,
        target_addr: SocketAddr,
        config: SenderConfig,
    ) -> Result<Self> {
        // Create audio format
        let format = AudioFormat {
            sample_rate: crate::config::format::SAMPLE_RATE,
            channels: crate::config::format::CHANNELS,
            bitrate: config.bitrate.as_u32(),
        };

        // Create encoder
        let encoder = OpusEncoder::new(format).map_err(AudioIoError::Codec)?;

        // Create RTP sender (async initialization)
        let rtp_sender = RtpSender::new(target_addr).await.map_err(AudioIoError::Codec)?;

        Ok(Self {
            capture,
            encoder,
            rtp_sender,
            config,
            running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(Mutex::new(SenderStats::default())),
            sequence_number: 0,
        })
    }

    /// Get a handle to control the running state
    pub fn running_handle(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }

    /// Get current statistics
    pub fn stats(&self) -> SenderStats {
        self.stats.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::AudioCapture;
    use std::net::{IpAddr, Ipv4Addr};

    struct MockCapture;

    impl AudioCapture for MockCapture {
        fn start(&mut self) -> Result<()> {
            Ok(())
        }

        fn stop(&mut self) -> Result<()> {
            Ok(())
        }

        fn read(&mut self, _buffer: &mut [i16]) -> Result<usize> {
            Ok(0)
        }
    }

    #[tokio::test]
    async fn test_sender_construction() {
        let capture = Box::new(MockCapture);
        let target_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);
        let config = SenderConfig::default();

        let sender = AudioSender::with_capture(capture, target_addr, config).await;
        assert!(sender.is_ok());

        let sender = sender.unwrap();
        assert_eq!(sender.sequence_number, 0);
        assert!(!sender.running.load(std::sync::atomic::Ordering::SeqCst));

        let stats = sender.stats();
        assert_eq!(stats.packets_sent, 0);
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.frames_captured, 0);
        assert_eq!(stats.encoding_errors, 0);
    }
}
