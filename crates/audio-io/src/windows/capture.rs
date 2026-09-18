//! WASAPI audio capture implementation for Windows
//!
//! Direct WASAPI COM API calls using windows-rs.

use crate::{traits::AudioCapture, AudioIoError, Result};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use windows::{
    core::*,
    Win32::Media::Audio::*,
    Win32::Media::KernelStreaming::*,
    Win32::System::Com::*,
};

/// WASAPI audio capture device wrapper
pub struct WasapiCapture {
    /// Shared buffer for captured audio
    buffer: Arc<Mutex<VecDeque<i16>>>,

    /// Whether capture is currently running
    running: Arc<Mutex<bool>>,
}

impl WasapiCapture {
    /// Create a new WASAPI capture device
    pub fn new(_device_name: Option<&str>) -> Result<Self> {
        Ok(Self {
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(48000 * 2))),
            running: Arc::new(Mutex::new(false)),
        })
    }
}

impl AudioCapture for WasapiCapture {
    fn start(&mut self) -> Result<()> {
        tracing::info!("WasapiCapture::start() called");

        {
            let mut running = self.running.lock().unwrap();
            if *running {
                tracing::warn!("Already running, skipping");
                return Ok(());
            }
            *running = true;
        }

        let buffer = Arc::clone(&self.buffer);
        let running = Arc::clone(&self.running);

        // Spawn capture thread
        std::thread::spawn(move || {
            if let Err(e) = capture_thread(buffer, running) {
                tracing::error!("Capture thread failed: {:?}", e);
            }
        });

        tracing::info!("Capture thread spawned");
        Ok(())
    }

    fn read(&mut self, buffer: &mut [i16]) -> Result<usize> {
        let running = self.running.lock().unwrap();
        if !*running {
            return Err(AudioIoError::Platform("Capture not started".into()));
        }
        drop(running);

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
        let mut running = self.running.lock().unwrap();
        if !*running {
            return Ok(());
        }

        *running = false;

        if let Ok(mut buf) = self.buffer.lock() {
            buf.clear();
        }

        Ok(())
    }
}

impl Drop for WasapiCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn capture_thread(
    buffer: Arc<Mutex<VecDeque<i16>>>,
    running: Arc<Mutex<bool>>,
) -> windows::core::Result<()> {
    tracing::info!("Capture thread started, initializing COM...");

    // Initialize COM
    unsafe {
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        if hr.is_err() {
            tracing::error!("Failed to initialize COM: {:?}", hr);
            return Err(hr.into());
        }
    }

    tracing::info!("COM initialized");

    let result = (|| -> windows::core::Result<()> {
        // Get device enumerator
        let enumerator: IMMDeviceEnumerator =
            unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };

        tracing::info!("Got device enumerator");

        // Get default audio endpoint (render device for loopback)
        let device = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eConsole)? };

        tracing::info!("Got default render device");

        // Activate audio client
        let audio_client: IAudioClient =
            unsafe { device.Activate(CLSCTX_ALL, None)? };

        tracing::info!("Got audio client");

        // Get mix format
        let format_ptr = unsafe { audio_client.GetMixFormat()? };
        let format = unsafe { &*format_ptr };

        // Copy fields to avoid packed struct reference issues
        let sample_rate = format.nSamplesPerSec;
        let channels = format.nChannels;
        let format_tag = format.wFormatTag;
        let bits_per_sample = format.wBitsPerSample;

        tracing::info!("Got mix format: {} Hz, {} channels", sample_rate, channels);

        // Initialize audio client in loopback mode
        unsafe {
            audio_client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_LOOPBACK,
                10_000_000, // 1 second buffer
                0,
                format_ptr,
                None,
            )?;
        }

        tracing::info!("Audio client initialized");

        // Get capture client
        let capture_client: IAudioCaptureClient = unsafe { audio_client.GetService()? };

        tracing::info!("Got capture client");

        // Start the audio stream
        unsafe { audio_client.Start()? };

        tracing::info!("Audio stream started, entering capture loop");

        // Capture loop
        loop {
            // Check if we should stop
            {
                let r = running.lock().unwrap();
                if !*r {
                    tracing::info!("Stop requested, exiting capture loop");
                    break;
                }
            }

            // Wait a bit
            std::thread::sleep(std::time::Duration::from_millis(10));

            // Get next packet size
            let packet_length = unsafe {
                capture_client.GetNextPacketSize()?
            };

            if packet_length == 0 {
                continue;
            }

            // Read the packet
            let mut data_ptr = std::ptr::null_mut();
            let mut num_frames_available = 0u32;
            let mut flags = 0u32;

            unsafe {
                capture_client.GetBuffer(
                    &mut data_ptr,
                    &mut num_frames_available,
                    &mut flags,
                    None,
                    None,
                )?;
            }

            if num_frames_available == 0 {
                unsafe {
                    capture_client.ReleaseBuffer(num_frames_available)?;
                }
                continue;
            }

            // Convert to i16 samples
            let num_samples = (num_frames_available * channels as u32) as usize;

            // Check format: 3 = WAVE_FORMAT_IEEE_FLOAT, 0xFFFE = WAVE_FORMAT_EXTENSIBLE
            if format_tag == 3 || (format_tag == 0xFFFE && bits_per_sample == 32) {
                // f32 samples
                let float_samples = unsafe {
                    std::slice::from_raw_parts(data_ptr as *const f32, num_samples)
                };

                if let Ok(mut buf) = buffer.lock() {
                    for &sample in float_samples {
                        let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                        buf.push_back(sample_i16);
                    }
                }
            } else {
                // i16 samples
                let int_samples = unsafe {
                    std::slice::from_raw_parts(data_ptr as *const i16, num_samples)
                };

                if let Ok(mut buf) = buffer.lock() {
                    for &sample in int_samples {
                        buf.push_back(sample);
                    }
                }
            }

            // Release the buffer
            unsafe {
                capture_client.ReleaseBuffer(num_frames_available)?;
            }
        }

        // Stop the stream
        unsafe { audio_client.Stop()? };

        tracing::info!("Capture thread stopping");

        Ok(())
    })();

    // Cleanup COM
    unsafe {
        CoUninitialize();
    }

    result
}
