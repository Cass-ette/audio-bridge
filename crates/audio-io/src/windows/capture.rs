//! WASAPI audio capture implementation for Windows
//!
//! Uses wasapi-rs for WASAPI loopback capture.

use crate::{traits::AudioCapture, AudioIoError, Result};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

/// WASAPI audio capture device
///
/// Captures system audio using WASAPI loopback mode via wasapi-rs library.
pub struct WasapiCapture {
    /// Shared buffer for captured audio
    buffer: Arc<Mutex<VecDeque<i16>>>,

    /// Whether capture is currently running
    running: bool,
}

impl WasapiCapture {
    /// Create a new WASAPI capture device
    pub fn new(_device_name: Option<&str>) -> Result<Self> {
        Ok(Self {
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(48000 * 2))),
            running: false,
        })
    }
}

impl AudioCapture for WasapiCapture {
    fn start(&mut self) -> Result<()> {
        if self.running {
            return Ok(());
        }

        use wasapi::*;

        // Initialize audio client for loopback capture
        let device = get_default_device(&Direction::Render)
            .map_err(|e| AudioIoError::DeviceNotFound(format!("Failed to get default render device: {}", e)))?;

        let mut audio_client = device.get_iaudioclient()
            .map_err(|e| AudioIoError::DeviceOpenFailed(format!("Failed to get audio client: {}", e)))?;

        // Get the mix format
        let waveformat = audio_client.get_mixformat()
            .map_err(|e| AudioIoError::DeviceOpenFailed(format!("Failed to get mix format: {}", e)))?;

        tracing::info!("Device format: {:?}", waveformat);

        // Initialize in shared mode with loopback
        let blockalign = waveformat.get_blockalign();
        let (def_time, min_time) = audio_client.get_periods()
            .map_err(|e| AudioIoError::Platform(format!("Failed to get periods: {}", e)))?;

        audio_client.initialize_client(
            &waveformat,
            def_time,
            &Direction::Capture,  // Use Capture direction for loopback
            &ShareMode::Shared,
            true  // Enable loopback
        ).map_err(|e| AudioIoError::DeviceOpenFailed(format!("Failed to initialize client: {}", e)))?;

        let buffer_frame_count = audio_client.get_bufferframecount()
            .map_err(|e| AudioIoError::Platform(format!("Failed to get buffer frame count: {}", e)))?;

        let render_client = audio_client.get_audiocaptureclient()
            .map_err(|e| AudioIoError::DeviceOpenFailed(format!("Failed to get capture client: {}", e)))?;

        let event = audio_client.set_get_eventhandle()
            .map_err(|e| AudioIoError::Platform(format!("Failed to set event handle: {}", e)))?;

        // Start the audio client
        audio_client.start_stream()
            .map_err(|e| AudioIoError::Platform(format!("Failed to start stream: {}", e)))?;

        let buffer = Arc::clone(&self.buffer);

        // Spawn a thread to capture audio
        std::thread::spawn(move || {
            let mut capture_client = render_client;

            loop {
                // Wait for data
                event.wait_for_event(1000).ok();

                // Get available frames
                if let Ok(nbr_frames) = capture_client.get_next_nbr_frames() {
                    if nbr_frames == 0 {
                        continue;
                    }

                    // Read data
                    if let Ok(data) = capture_client.read_from_device(nbr_frames) {
                        // Convert to i16 samples
                        if let Ok(mut buf) = buffer.lock() {
                            // wasapi-rs returns f32 samples, convert to i16
                            for sample in data.iter() {
                                let sample_i16 = (*sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                                buf.push_back(sample_i16);
                            }
                        }
                    }
                }
            }
        });

        self.running = true;
        Ok(())
    }

    fn read(&mut self, buffer: &mut [i16]) -> Result<usize> {
        if !self.running {
            return Err(AudioIoError::Platform("Capture not started".into()));
        }

        let mut buf = self.buffer.lock()
            .map_err(|_| AudioIoError::Platform("Failed to lock buffer".into()))?;

        let samples_available = buf.len();
        if samples_available == 0 {
            return Ok(0);
        }

        let samples_to_copy = samples_available.min(buffer.len());
        for i in 0..samples_to_copy {
            buffer[i] = buf.pop_front().unwrap();
        }

        Ok(samples_to_copy)
    }

    fn stop(&mut self) -> Result<()> {
        if !self.running {
            return Ok(());
        }

        // Clear buffer
        if let Ok(mut buf) = self.buffer.lock() {
            buf.clear();
        }

        self.running = false;
        Ok(())
    }
}

impl Drop for WasapiCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
