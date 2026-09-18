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
    /// Create a new AudioReceiver (macOS default)
    ///
    /// On macOS, this automatically creates a CoreAudioPlayback device.
    #[cfg(target_os = "macos")]
    pub async fn new(bind_addr: SocketAddr, config: ReceiverConfig) -> Result<Self> {
        use crate::macos::CoreAudioPlayback;
        let playback = Box::new(CoreAudioPlayback::new()?);
        Self::with_playback(playback, bind_addr, config).await
    }

    /// Create a new AudioReceiver with a provided playback device
    pub async fn with_playback(
        playback: Box<dyn AudioPlayback>,
        bind_addr: SocketAddr,
        config: ReceiverConfig,
    ) -> Result<Self> {
        // Create audio format
        let format = AudioFormat {
            sample_rate: crate::config::format::SAMPLE_RATE,
            channels: crate::config::format::CHANNELS,
            bitrate: config.bitrate.as_u32(),
        };

        // Create decoder
        let decoder = OpusDecoder::new(format).map_err(AudioIoError::Codec)?;

        // Create RTP receiver (async initialization)
        let rtp_receiver = RtpReceiver::new(bind_addr).await.map_err(AudioIoError::Codec)?;

        // Create jitter buffer
        let jitter_buffer = JitterBuffer::new(config.jitter_buffer_size);

        Ok(Self {
            rtp_receiver,
            jitter_buffer,
            decoder,
            playback,
            config,
            running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(Mutex::new(ReceiverStats::default())),
        })
    }

    /// Get a handle to control the running state
    pub fn running_handle(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }

    /// Get current statistics
    pub fn stats(&self) -> ReceiverStats {
        self.stats.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::AudioPlayback;
    use std::net::{IpAddr, Ipv4Addr};

    struct MockPlayback;

    impl AudioPlayback for MockPlayback {
        fn start(&mut self) -> Result<()> {
            Ok(())
        }

        fn stop(&mut self) -> Result<()> {
            Ok(())
        }

        fn write(&mut self, _buffer: &[i16]) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_receiver_construction() {
        let playback = Box::new(MockPlayback);
        let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8001);
        let config = ReceiverConfig::default();

        let receiver = AudioReceiver::with_playback(playback, bind_addr, config).await;
        assert!(receiver.is_ok());

        let receiver = receiver.unwrap();
        assert!(!receiver.running.load(Ordering::SeqCst));

        let stats = receiver.stats();
        assert_eq!(stats.packets_received, 0);
        assert_eq!(stats.packets_decoded, 0);
        assert_eq!(stats.packets_lost, 0);
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.decoding_errors, 0);
    }
}
