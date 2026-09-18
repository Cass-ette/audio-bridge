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
    #[allow(dead_code)]
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
        let rtp_receiver = RtpReceiver::new(bind_addr)
            .await
            .map_err(AudioIoError::Codec)?;

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

    /// Start the audio receiver pipeline
    ///
    /// Runs the receive → jitter buffer → decode → playback loop until stop() is called.
    /// The loop processes audio in 20ms frames (960 samples per channel at 48kHz).
    ///
    /// Returns Ok(()) when stopped gracefully, Err for fatal errors.
    pub async fn start(&mut self) -> Result<()> {
        // Set running flag
        self.running.store(true, Ordering::SeqCst);

        // Start playback device
        self.playback.start()?;

        while self.running.load(Ordering::SeqCst) {
            // Step 1: Receive RTP packet with timeout to allow checking running flag
            let packet = match tokio::time::timeout(
                tokio::time::Duration::from_millis(100),
                self.rtp_receiver.receive(),
            )
            .await
            {
                Ok(Ok(pkt)) => pkt,
                Ok(Err(e)) => {
                    eprintln!("RTP receive error (continuing): {:?}", e);
                    continue;
                }
                Err(_) => {
                    // Timeout - check running flag and continue
                    continue;
                }
            };

            // Update receive stats
            {
                let mut stats = self.stats.lock().unwrap();
                stats.packets_received += 1;
                stats.bytes_received += packet.payload.len() as u64;
            }

            // Step 2: Insert into jitter buffer
            self.jitter_buffer.insert(packet);

            // Step 3: Process packets from jitter buffer
            while let Some(buffered_packet) = self.jitter_buffer.pop() {
                // Step 4: Decode the packet
                let decoded = match self.decoder.decode(&buffered_packet.payload, false) {
                    Ok(samples) => {
                        // Update decode stats
                        let mut stats = self.stats.lock().unwrap();
                        stats.packets_decoded += 1;
                        samples
                    }
                    Err(e) => {
                        // Decoding failed - use PLC (Packet Loss Concealment)
                        eprintln!("Decoding error, using PLC: {:?}", e);
                        let mut stats = self.stats.lock().unwrap();
                        stats.decoding_errors += 1;

                        match self.decoder.decode_plc() {
                            Ok(samples) => samples,
                            Err(plc_err) => {
                                eprintln!("PLC failed: {:?}", plc_err);
                                continue;
                            }
                        }
                    }
                };

                // Step 5: Write to playback (with retry on buffer overrun)
                let mut retries = 0;
                loop {
                    match self.playback.write(&decoded) {
                        Ok(_) => break,
                        Err(AudioIoError::BufferOverrun) if retries < 3 => {
                            // Buffer full - wait a bit and retry
                            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
                            retries += 1;
                        }
                        Err(e) => {
                            eprintln!("Playback write error (continuing): {:?}", e);
                            break;
                        }
                    }
                }
            }

            // Update packet loss stats from jitter buffer
            {
                let mut stats = self.stats.lock().unwrap();
                let jitter_stats = self.jitter_buffer.stats();
                stats.packets_lost = jitter_stats.packets_lost;
            }
        }

        // Stop playback device
        self.playback.stop()?;

        Ok(())
    }

    /// Stop the audio receiver pipeline
    ///
    /// Sets the running flag to false, causing start() to exit after the current iteration.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
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

    #[tokio::test]
    async fn test_end_to_end_loopback() {
        use crate::mock::FileAudioSource;
        use crate::sender::AudioSender;
        use crate::SenderConfig;
        use std::path::PathBuf;
        use std::time::Duration;

        // Create a test WAV file with 50 frames (20ms each = 1 second total)
        let test_file = PathBuf::from("/tmp/test_e2e_loopback.wav");
        create_test_wav(&test_file, 50);

        // Setup addresses
        let receiver_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9000);

        // Create receiver with mock playback (avoids CoreAudio buffer issues in tests)
        let playback = Box::new(MockPlayback);
        let mut receiver =
            AudioReceiver::with_playback(playback, receiver_addr, ReceiverConfig::default())
                .await
                .unwrap();

        // Create sender with file source
        let capture = Box::new(FileAudioSource::open(test_file.clone()).unwrap());
        let mut sender = AudioSender::with_capture(capture, receiver_addr, SenderConfig::default())
            .await
            .unwrap();

        // Get handles before moving
        let receiver_running = receiver.running_handle();
        let sender_running = sender.running_handle();
        let receiver_stats_handle = receiver.stats.clone();

        // Start receiver in background task
        let receiver_task = tokio::spawn(async move { receiver.start().await });

        // Start sender in background thread (blocking API)
        let sender_thread = std::thread::spawn(move || sender.start());

        // Let it run for 1.5 seconds (enough to process all 50 frames)
        tokio::time::sleep(Duration::from_millis(1500)).await;

        // Stop both
        sender_running.store(false, Ordering::SeqCst);
        receiver_running.store(false, Ordering::SeqCst);

        // Wait for completion
        let receiver_result = receiver_task.await.unwrap();
        let sender_result = sender_thread.join().unwrap();

        // Verify both completed successfully
        assert!(
            sender_result.is_ok(),
            "Sender failed: {:?}",
            sender_result.err()
        );
        assert!(
            receiver_result.is_ok(),
            "Receiver failed: {:?}",
            receiver_result.err()
        );

        // Verify stats show activity
        let receiver_final_stats = receiver_stats_handle.lock().unwrap().clone();

        println!(
            "Receiver stats: packets_received={}, packets_decoded={}, packets_lost={}",
            receiver_final_stats.packets_received,
            receiver_final_stats.packets_decoded,
            receiver_final_stats.packets_lost
        );

        assert!(
            receiver_final_stats.packets_received > 0,
            "Expected packets_received > 0"
        );
        assert!(
            receiver_final_stats.packets_decoded > 0,
            "Expected packets_decoded > 0"
        );
    }

    // Helper to create a test WAV file with N frames
    fn create_test_wav(path: &std::path::Path, num_frames: usize) {
        use std::f32::consts::PI;
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

        // Write a 440Hz sine wave (audible tone)
        let frequency = 440.0;
        let amplitude = 0.3; // 30% volume
        for i in 0..total_samples / 2 {
            let t = i as f32 / sample_rate as f32;
            let sample = (2.0 * PI * frequency * t).sin() * amplitude * i16::MAX as f32;
            let sample_i16 = sample as i16;
            // Write stereo (L, R)
            file.write_all(&sample_i16.to_le_bytes()).unwrap();
            file.write_all(&sample_i16.to_le_bytes()).unwrap();
        }
    }
}
