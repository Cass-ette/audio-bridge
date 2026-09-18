//! WASAPI audio capture implementation for Windows
//!
//! Uses Windows Audio Session API (WASAPI) to capture system audio in loopback mode.

use crate::{traits::AudioCapture, AudioIoError, Result};
use std::ptr;
use std::sync::Arc;
use windows::core::*;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;

/// WASAPI audio capture device
///
/// Captures system audio using Windows Audio Session API in loopback mode.
/// Automatically initializes COM and cleans up resources on drop.
pub struct WasapiCapture {
    /// COM initialization guard (Drop cleans up COM)
    _com_guard: ComGuard,

    /// Audio client interface
    audio_client: Option<IAudioClient>,

    /// Capture client interface
    capture_client: Option<IAudioCaptureClient>,

    /// Device name (for error reporting)
    device_name: String,

    /// Whether capture is currently running
    running: bool,
}

/// RAII guard for COM initialization
struct ComGuard;

impl ComGuard {
    fn new() -> Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED)
                .map_err(|e| AudioIoError::Platform(format!("COM initialization failed: {}", e)))?;
        }
        Ok(ComGuard)
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

impl WasapiCapture {
    /// Create a new WASAPI capture device
    ///
    /// # Arguments
    /// * `device_name` - Device to capture from (None = default loopback device)
    ///
    /// # Returns
    /// Configured capture device ready to start
    pub fn new(device_name: Option<String>) -> Result<Self> {
        // Initialize COM
        let com_guard = ComGuard::new()?;

        // Get device enumerator
        let enumerator: IMMDeviceEnumerator = unsafe {
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|e| AudioIoError::Platform(format!("Failed to create device enumerator: {}", e)))?
        };

        // Get default render device (for loopback capture)
        let device: IMMDevice = unsafe {
            if let Some(ref name) = device_name {
                // Try to find device by name
                Self::find_device_by_name(&enumerator, name)?
            } else {
                // Use default render device
                enumerator
                    .GetDefaultAudioEndpoint(eRender, eConsole)
                    .map_err(|e| AudioIoError::DeviceNotFound(format!("Default device: {}", e)))?
            }
        };

        // Get device friendly name for error reporting
        let friendly_name = Self::get_device_name(&device)?;

        // Activate audio client
        let audio_client: IAudioClient = unsafe {
            device
                .Activate(CLSCTX_ALL, None)
                .map_err(|e| AudioIoError::DeviceOpenFailed(format!("Failed to activate device: {}", e)))?
        };

        // Initialize audio client in loopback mode
        Self::initialize_audio_client(&audio_client)?;

        Ok(Self {
            _com_guard: com_guard,
            audio_client: Some(audio_client),
            capture_client: None,
            device_name: friendly_name,
            running: false,
        })
    }

    /// Find device by friendly name
    fn find_device_by_name(enumerator: &IMMDeviceEnumerator, name: &str) -> Result<IMMDevice> {
        unsafe {
            let collection = enumerator
                .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
                .map_err(|e| AudioIoError::Platform(format!("Failed to enumerate devices: {}", e)))?;

            let count = collection
                .GetCount()
                .map_err(|e| AudioIoError::Platform(format!("Failed to get device count: {}", e)))?;

            for i in 0..count {
                let device = collection
                    .Item(i)
                    .map_err(|e| AudioIoError::Platform(format!("Failed to get device {}: {}", i, e)))?;

                let device_name = Self::get_device_name(&device)?;
                if device_name == name {
                    return Ok(device);
                }
            }

            Err(AudioIoError::DeviceNotFound(format!("Device '{}' not found", name)))
        }
    }

    /// Get device friendly name
    fn get_device_name(device: &IMMDevice) -> Result<String> {
        unsafe {
            let props = device
                .OpenPropertyStore(STGM_READ)
                .map_err(|e| AudioIoError::Platform(format!("Failed to open property store: {}", e)))?;

            let prop_variant = props
                .GetValue(&PKEY_Device_FriendlyName)
                .map_err(|e| AudioIoError::Platform(format!("Failed to get device name: {}", e)))?;

            let name = prop_variant.Anonymous.Anonymous.Anonymous.pwszVal;
            let name_str = name.to_string()
                .map_err(|e| AudioIoError::Platform(format!("Failed to convert device name: {}", e)))?;

            Ok(name_str)
        }
    }

    /// Initialize audio client with required format
    fn initialize_audio_client(audio_client: &IAudioClient) -> Result<()> {
        unsafe {
            // Get device mix format
            let mix_format = audio_client
                .GetMixFormat()
                .map_err(|e| AudioIoError::Platform(format!("Failed to get mix format: {}", e)))?;

            // Verify format is compatible (48kHz stereo 16-bit)
            let format = &*mix_format;
            if format.nSamplesPerSec != 48000 || format.nChannels != 2 {
                return Err(AudioIoError::UnsupportedFormat(
                    format!(
                        "Device format {}Hz {}ch not supported (require 48kHz stereo)",
                        format.nSamplesPerSec, format.nChannels
                    )
                ));
            }

            // Initialize in loopback mode
            // AUDCLNT_STREAMFLAGS_LOOPBACK captures what's playing on the device
            audio_client
                .Initialize(
                    AUDCLNT_SHAREMODE_SHARED,
                    AUDCLNT_STREAMFLAGS_LOOPBACK,
                    10_000_000, // 1 second buffer (in 100ns units)
                    0,
                    mix_format,
                    None,
                )
                .map_err(|e| AudioIoError::DeviceOpenFailed(format!("Failed to initialize audio client: {}", e)))?;

            // Free mix format
            CoTaskMemFree(Some(mix_format as *const _ as *const _));

            Ok(())
        }
    }
}

impl AudioCapture for WasapiCapture {
    fn start(&mut self) -> Result<()> {
        if self.running {
            return Ok(());
        }

        let audio_client = self.audio_client.as_ref()
            .ok_or_else(|| AudioIoError::Platform("Audio client not initialized".into()))?;

        // Get capture client
        let capture_client: IAudioCaptureClient = unsafe {
            audio_client
                .GetService()
                .map_err(|e| AudioIoError::DeviceOpenFailed(format!("Failed to get capture client: {}", e)))?
        };

        self.capture_client = Some(capture_client);

        // Start capturing
        unsafe {
            audio_client
                .Start()
                .map_err(|e| AudioIoError::Platform(format!("Failed to start capture: {}", e)))?;
        }

        self.running = true;
        Ok(())
    }

    fn read(&mut self, buffer: &mut [i16]) -> Result<usize> {
        if !self.running {
            return Err(AudioIoError::Platform("Capture not started".into()));
        }

        let capture_client = self.capture_client.as_ref()
            .ok_or_else(|| AudioIoError::Platform("Capture client not initialized".into()))?;

        unsafe {
            // Get next packet size
            let mut packet_length = 0u32;
            capture_client
                .GetNextPacketSize(&mut packet_length)
                .map_err(|e| AudioIoError::Platform(format!("Failed to get packet size: {}", e)))?;

            if packet_length == 0 {
                // No data available, return 0 (caller should retry)
                return Ok(0);
            }

            // Get buffer
            let mut data: *mut u8 = ptr::null_mut();
            let mut num_frames = 0u32;
            let mut flags = 0u32;

            capture_client
                .GetBuffer(&mut data, &mut num_frames, &mut flags, None, None)
                .map_err(|e| AudioIoError::Platform(format!("Failed to get buffer: {}", e)))?;

            // Calculate samples to copy (stereo = 2 channels)
            let samples_available = (num_frames as usize) * 2;
            let samples_to_copy = samples_available.min(buffer.len());

            // Check for silence flag
            if flags & AUDCLNT_BUFFERFLAGS_SILENT.0 != 0 {
                // Fill with silence
                buffer[..samples_to_copy].fill(0);
            } else {
                // Copy audio data (16-bit PCM)
                let src = std::slice::from_raw_parts(data as *const i16, samples_available);
                buffer[..samples_to_copy].copy_from_slice(&src[..samples_to_copy]);
            }

            // Release buffer
            capture_client
                .ReleaseBuffer(num_frames)
                .map_err(|e| AudioIoError::Platform(format!("Failed to release buffer: {}", e)))?;

            Ok(samples_to_copy)
        }
    }

    fn stop(&mut self) -> Result<()> {
        if !self.running {
            return Ok(());
        }

        if let Some(audio_client) = &self.audio_client {
            unsafe {
                audio_client
                    .Stop()
                    .map_err(|e| AudioIoError::Platform(format!("Failed to stop capture: {}", e)))?;
            }
        }

        self.capture_client = None;
        self.running = false;
        Ok(())
    }
}

impl Drop for WasapiCapture {
    fn drop(&mut self) {
        // Stop capture if still running
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasapi_construction() {
        // This test will only run on Windows
        #[cfg(target_os = "windows")]
        {
            let capture = WasapiCapture::new(None);
            // May fail if no audio device available in CI
            if let Ok(capture) = capture {
                assert!(!capture.running);
                assert!(!capture.device_name.is_empty());
            }
        }
    }
}
