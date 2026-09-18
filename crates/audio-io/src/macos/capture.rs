//! CoreAudio capture implementation for macOS
//!
//! Uses cpal for CoreAudio microphone capture.

use crate::{traits::AudioCapture, AudioIoError, Result};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

/// CoreAudio capture device wrapper
pub struct CoreAudioCapture {
    /// Shared buffer for captured audio
    buffer: Arc<Mutex<VecDeque<i16>>>,

    /// Whether capture is currently running
    running: Arc<Mutex<bool>>,
}

impl CoreAudioCapture {
    /// Create a new CoreAudio capture device
    pub fn new(_device_name: Option<&str>) -> Result<Self> {
        Ok(Self {
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(48000 * 2))),
            running: Arc::new(Mutex::new(false)),
        })
    }
}

impl AudioCapture for CoreAudioCapture {
    fn start(&mut self) -> Result<()> {
        tracing::info!("CoreAudioCapture::start() called");

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

        // Spawn capture thread - cpal Stream must stay in this thread
        std::thread::spawn(move || {
            use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

            tracing::info!("CoreAudio capture thread started");

            // Get host
            let host = cpal::default_host();

            // Get default input device (microphone)
            let device = match host.default_input_device() {
                Some(d) => d,
                None => {
                    tracing::error!("No default input device");
                    return;
                }
            };

            tracing::info!("Got default input device: {:?}", device.name());

            // Get supported config
            let config = match device.default_input_config() {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Failed to get default config: {}", e);
                    return;
                }
            };

            tracing::info!("Device config: {:?}", config);

            let sample_format = config.sample_format();
            tracing::info!("Sample format: {:?}", sample_format);

            let buffer_clone = Arc::clone(&buffer);

            // Build input stream based on sample format
            let stream = match sample_format {
                cpal::SampleFormat::F32 => {
                    device.build_input_stream(
                        &config.into(),
                        move |data: &[f32], _: &cpal::InputCallbackInfo| {
                            // Convert f32 to i16 and push to buffer
                            if let Ok(mut buf) = buffer_clone.lock() {
                                for &sample in data {
                                    let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                                    buf.push_back(sample_i16);
                                }
                            }
                        },
                        |err| {
                            tracing::error!("Stream error: {}", err);
                        },
                        None,
                    )
                }
                cpal::SampleFormat::I16 => {
                    device.build_input_stream(
                        &config.into(),
                        move |data: &[i16], _: &cpal::InputCallbackInfo| {
                            // Copy i16 samples directly
                            if let Ok(mut buf) = buffer_clone.lock() {
                                buf.extend(data);
                            }
                        },
                        |err| {
                            tracing::error!("Stream error: {}", err);
                        },
                        None,
                    )
                }
                _ => {
                    tracing::error!("Unsupported sample format: {:?}", sample_format);
                    return;
                }
            };

            let stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to build input stream: {}", e);
                    return;
                }
            };

            // Start the stream
            if let Err(e) = stream.play() {
                tracing::error!("Failed to start stream: {}", e);
                return;
            }

            tracing::info!("CoreAudio stream started, entering loop");

            // Keep stream alive until stop is requested
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));

                let r = running.lock().unwrap();
                if !*r {
                    tracing::info!("Stop requested, exiting");
                    break;
                }
            }

            // Stream is automatically stopped when dropped
            tracing::info!("CoreAudio capture thread stopped");
        });

        tracing::info!("CoreAudio capture thread spawned");
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

impl Drop for CoreAudioCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
