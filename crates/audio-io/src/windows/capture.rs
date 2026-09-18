//! WASAPI audio capture implementation for Windows
//!
//! Uses cpal for WASAPI loopback capture.

use crate::{traits::AudioCapture, AudioIoError, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

/// WASAPI audio capture device
///
/// Captures system audio using WASAPI loopback mode via cpal.
/// The stream is leaked to keep it running, so it must be manually stopped.
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

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| AudioIoError::DeviceNotFound("No default output device".into()))?;

        let config = device
            .default_input_config()
            .map_err(|e| AudioIoError::DeviceOpenFailed(format!("Failed to get config: {}", e)))?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();

        if sample_rate != 48000 {
            tracing::warn!("Device sample rate {}Hz != 48kHz", sample_rate);
        }
        if channels != 2 {
            return Err(AudioIoError::UnsupportedFormat(format!(
                "Device has {} channels, require 2",
                channels
            )));
        }

        let stream_config = config.config();
        let buffer = Arc::clone(&self.buffer);

        let stream = device
            .build_input_stream(
                &stream_config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut buf) = buffer.lock() {
                        buf.extend(data.iter().copied());
                    }
                },
                |err| {
                    tracing::error!("WASAPI stream error: {}", err);
                },
                None,
            )
            .map_err(|e| AudioIoError::DeviceOpenFailed(format!("Failed to build stream: {}", e)))?;

        stream
            .play()
            .map_err(|e| AudioIoError::Platform(format!("Failed to start stream: {}", e)))?;

        // Leak the stream to keep it running
        // This is necessary because cpal::Stream is !Send and we can't store it
        std::mem::forget(stream);

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

        // Note: We can't actually stop the leaked stream
        // This is a limitation of the current design
        // The stream will continue running until process exit

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
