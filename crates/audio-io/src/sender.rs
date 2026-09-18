use crate::{traits::AudioCapture, AudioIoError, Result, SenderConfig};
use audio_bridge_core::codec::{AudioFormat, OpusEncoder};
use audio_bridge_core::rtp::{RtpHeader, RtpPacket};
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
    #[allow(dead_code)]
    config: SenderConfig,
    running: Arc<AtomicBool>,
    stats: Arc<Mutex<SenderStats>>,
    sequence_number: u16,
}

impl AudioSender {
    /// Create a new AudioSender with platform-default capture device
    ///
    /// # Platform Support
    /// - Windows: Uses WASAPI loopback capture
    /// - macOS: Not implemented (use `with_capture()` instead)
    /// - Linux: Not implemented (use `with_capture()` instead)
    #[cfg(target_os = "windows")]
    pub async fn new(target_addr: SocketAddr, config: SenderConfig) -> Result<Self> {
        let capture = Box::new(crate::windows::WasapiCapture::new(
            config.capture_device.as_deref(),
        )?);
        Self::with_capture(capture, target_addr, config).await
    }

    /// Create a new AudioSender with platform-default capture device
    ///
    /// # Platform Support
    /// - Windows: Uses WASAPI loopback capture
    /// - macOS: Not implemented (use `with_capture()` instead)
    /// - Linux: Not implemented (use `with_capture()` instead)
    #[cfg(not(target_os = "windows"))]
    pub async fn new(_target_addr: SocketAddr, _config: SenderConfig) -> Result<Self> {
        Err(AudioIoError::Platform(
            "Platform-default capture not available. Use with_capture()".into(),
        ))
    }

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
        let rtp_sender = RtpSender::new(target_addr)
            .await
            .map_err(AudioIoError::Codec)?;

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

    /// Start the audio sender pipeline (blocking)
    ///
    /// Runs the capture → encode → RTP → send loop until stop() is called or a fatal error occurs.
    /// The loop processes audio in 20ms frames (960 samples per channel at 48kHz).
    ///
    /// Returns Ok(()) when stopped gracefully, Err for fatal errors.
    pub fn start(&mut self) -> Result<()> {
        // Set running flag
        self.running.store(true, Ordering::SeqCst);

        // Start capture device
        self.capture.start()?;

        // Calculate frame size (20ms at 48kHz stereo = 960 * 2 = 1920 samples)
        let frame_size = (crate::config::format::SAMPLE_RATE / 50) as usize; // 960 samples per channel
        let samples_per_frame = frame_size * crate::config::format::CHANNELS as usize; // 1920 total
        let mut buffer = vec![0i16; samples_per_frame];

        // Initialize RTP header (will be reused with incrementing sequence/timestamp)
        let ssrc = 0x12345678u32; // Fixed SSRC for this session
        let payload_type = 96u8; // Dynamic payload type for Opus

        // Create or get tokio runtime for async operations
        let runtime = match tokio::runtime::Handle::try_current() {
            Ok(_handle) => None, // Already in a runtime context
            Err(_) => {
                // Create a new runtime for this thread
                Some(tokio::runtime::Runtime::new().map_err(|e| {
                    AudioIoError::Platform(format!("Failed to create tokio runtime: {}", e))
                })?)
            }
        };

        while self.running.load(Ordering::SeqCst) {
            // Step 1: Capture audio from device
            let samples_read = match self.capture.read(&mut buffer) {
                Ok(n) => n,
                Err(e) => {
                    // Fatal errors: break immediately
                    if e.is_fatal() {
                        return Err(e);
                    }
                    // Temporary errors: log and continue
                    eprintln!("Capture error (continuing): {:?}", e);
                    continue;
                }
            };

            // If no samples read, EOF reached
            if samples_read == 0 {
                break; // Graceful exit
            }

            // Update frames captured stat
            {
                let mut stats = self.stats.lock().unwrap();
                stats.frames_captured += 1;
            }

            // Step 2: Encode with Opus
            let encoded = match self.encoder.encode(&buffer[..samples_read]) {
                Ok(data) => data,
                Err(e) => {
                    // Encoding error: log and continue
                    eprintln!("Encoding error (continuing): {:?}", e);
                    let mut stats = self.stats.lock().unwrap();
                    stats.encoding_errors += 1;
                    continue;
                }
            };

            // Step 3: Create RTP packet
            let header = RtpHeader {
                version: 2,
                padding: false,
                extension: false,
                csrc_count: 0,
                marker: false,
                payload_type,
                sequence_number: self.sequence_number,
                timestamp: (self.sequence_number as u32).wrapping_mul(frame_size as u32),
                ssrc,
            };
            let packet = RtpPacket::new(header, encoded);

            // Step 4: Send via RTP (async operation)
            let send_result = if let Some(ref rt) = runtime {
                // Use our own runtime
                rt.block_on(async {
                    self.rtp_sender
                        .send(&packet)
                        .await
                        .map_err(AudioIoError::Codec)
                })?
            } else {
                // Use existing runtime handle
                tokio::runtime::Handle::current().block_on(async {
                    self.rtp_sender
                        .send(&packet)
                        .await
                        .map_err(AudioIoError::Codec)
                })?
            };

            // Step 5: Update stats
            {
                let mut stats = self.stats.lock().unwrap();
                stats.packets_sent += 1;
                stats.bytes_sent += send_result as u64;
            }

            // Increment sequence number (wrapping)
            self.sequence_number = self.sequence_number.wrapping_add(1);
        }

        // Stop capture device
        self.capture.stop()?;

        Ok(())
    }

    /// Stop the audio sender pipeline
    ///
    /// Sets the running flag to false, causing start() to exit after the current frame.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
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

    #[tokio::test]
    async fn test_sender_pipeline() {
        use crate::mock::FileAudioSource;
        use std::path::PathBuf;
        use std::time::Duration;

        // Create a test WAV file with 5 frames (20ms each = 100ms total)
        let test_file = PathBuf::from("/tmp/test_sender_5frames.wav");
        create_test_wav(&test_file, 5);

        // Create sender with file capture
        let capture = Box::new(FileAudioSource::open(test_file.clone()).unwrap());
        let target_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);
        let config = SenderConfig::default();

        let mut sender = AudioSender::with_capture(capture, target_addr, config)
            .await
            .unwrap();

        // Get stats handle before moving sender
        let stats_handle = sender.stats.clone();

        // Run sender in background thread
        let running_handle = sender.running_handle();
        let sender_thread = std::thread::spawn(move || sender.start());

        // Let it run briefly
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Stop sender
        running_handle.store(false, std::sync::atomic::Ordering::SeqCst);
        let result = sender_thread.join().unwrap();
        if let Err(ref e) = result {
            panic!("Sender failed with error: {:?}", e);
        }
        assert!(result.is_ok());

        // Verify stats show activity
        let stats = stats_handle.lock().unwrap().clone();
        assert!(
            stats.packets_sent > 0,
            "Expected packets_sent > 0, got {}",
            stats.packets_sent
        );
        assert!(
            stats.bytes_sent > 0,
            "Expected bytes_sent > 0, got {}",
            stats.bytes_sent
        );
        assert!(
            stats.frames_captured > 0,
            "Expected frames_captured > 0, got {}",
            stats.frames_captured
        );
    }

    // Helper to create a test WAV file with N frames
    fn create_test_wav(path: &std::path::Path, num_frames: usize) {
        use std::fs::File;
        use std::io::Write;

        let sample_rate = 48000u32;
        let channels = 2u16;
        let bits_per_sample = 16u16;
        let samples_per_frame = 960 * channels as usize; // 20ms stereo
        let total_samples = samples_per_frame * num_frames;
        let data_size = (total_samples * 2) as u32; // 2 bytes per i16 sample

        let mut file = File::create(path).unwrap();

        // WAV header
        file.write_all(b"RIFF").unwrap();
        file.write_all(&(36 + data_size).to_le_bytes()).unwrap();
        file.write_all(b"WAVE").unwrap();

        // fmt chunk
        file.write_all(b"fmt ").unwrap();
        file.write_all(&16u32.to_le_bytes()).unwrap(); // chunk size
        file.write_all(&1u16.to_le_bytes()).unwrap(); // PCM format
        file.write_all(&channels.to_le_bytes()).unwrap();
        file.write_all(&sample_rate.to_le_bytes()).unwrap();
        let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
        file.write_all(&byte_rate.to_le_bytes()).unwrap();
        let block_align = channels * bits_per_sample / 8;
        file.write_all(&block_align.to_le_bytes()).unwrap();
        file.write_all(&bits_per_sample.to_le_bytes()).unwrap();

        // data chunk
        file.write_all(b"data").unwrap();
        file.write_all(&data_size.to_le_bytes()).unwrap();

        // Write silence (or simple waveform)
        for _ in 0..total_samples {
            file.write_all(&0i16.to_le_bytes()).unwrap();
        }
    }
}
