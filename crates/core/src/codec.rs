use crate::{AudioBridgeError, Result};
use opus::{Application, Channels, Decoder, Encoder};

/// Audio format specification
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioFormat {
    pub sample_rate: u32, // Always 48000 for Opus
    pub channels: u16,    // 1 (mono) or 2 (stereo)
    pub bitrate: u32,     // Target bitrate in bps (e.g., 128000)
}

/// Opus encoder wrapper
pub struct OpusEncoder {
    encoder: Encoder,
    format: AudioFormat,
}

impl OpusEncoder {
    /// Create new Opus encoder
    pub fn new(format: AudioFormat) -> Result<Self> {
        if format.sample_rate != 48000 {
            return Err(AudioBridgeError::InvalidFormat(format!(
                "Opus requires 48kHz sample rate, got {}",
                format.sample_rate
            )));
        }

        let channels = match format.channels {
            1 => Channels::Mono,
            2 => Channels::Stereo,
            n => {
                return Err(AudioBridgeError::InvalidFormat(format!(
                    "Unsupported channel count: {}",
                    n
                )))
            }
        };

        let mut encoder = Encoder::new(
            format.sample_rate,
            channels,
            Application::Audio, // General audio (not voip or restricted_lowdelay)
        )
        .map_err(|e| AudioBridgeError::OpusError(format!("Failed to create encoder: {:?}", e)))?;

        // Set bitrate
        encoder
            .set_bitrate(opus::Bitrate::Bits(format.bitrate as i32))
            .map_err(|e| AudioBridgeError::OpusError(format!("Failed to set bitrate: {:?}", e)))?;

        // Enable VBR (Variable Bit Rate)
        encoder
            .set_vbr(true)
            .map_err(|e| AudioBridgeError::OpusError(format!("Failed to enable VBR: {:?}", e)))?;

        Ok(Self { encoder, format })
    }

    /// Encode PCM samples to Opus
    ///
    /// `pcm`: Interleaved i16 samples (L, R, L, R, ...)
    /// Returns: Opus-encoded bytes
    pub fn encode(&mut self, pcm: &[i16]) -> Result<Vec<u8>> {
        // Calculate expected frame size (20ms = 960 samples at 48kHz)
        let frame_size = (self.format.sample_rate / 50) as usize; // 48000 / 50 = 960
        let expected_samples = frame_size * self.format.channels as usize;

        if pcm.len() != expected_samples {
            return Err(AudioBridgeError::InvalidFormat(format!(
                "Expected {} samples, got {}",
                expected_samples,
                pcm.len()
            )));
        }

        let mut output = vec![0u8; 4000]; // Max Opus packet size
        let len = self
            .encoder
            .encode(pcm, &mut output)
            .map_err(|e| AudioBridgeError::OpusError(format!("Encode failed: {:?}", e)))?;

        output.truncate(len);
        Ok(output)
    }

    pub fn format(&self) -> AudioFormat {
        self.format
    }
}

/// Opus decoder wrapper
pub struct OpusDecoder {
    decoder: Decoder,
    format: AudioFormat,
}

impl OpusDecoder {
    /// Create new Opus decoder
    pub fn new(format: AudioFormat) -> Result<Self> {
        if format.sample_rate != 48000 {
            return Err(AudioBridgeError::InvalidFormat(format!(
                "Opus requires 48kHz sample rate, got {}",
                format.sample_rate
            )));
        }

        let channels = match format.channels {
            1 => Channels::Mono,
            2 => Channels::Stereo,
            n => {
                return Err(AudioBridgeError::InvalidFormat(format!(
                    "Unsupported channel count: {}",
                    n
                )))
            }
        };

        let decoder = Decoder::new(format.sample_rate, channels).map_err(|e| {
            AudioBridgeError::OpusError(format!("Failed to create decoder: {:?}", e))
        })?;

        Ok(Self { decoder, format })
    }

    /// Decode Opus bytes to PCM
    ///
    /// `data`: Opus-encoded bytes
    /// `fec`: Use Forward Error Correction (for packet loss recovery)
    /// Returns: Interleaved i16 PCM samples
    pub fn decode(&mut self, data: &[u8], fec: bool) -> Result<Vec<i16>> {
        let frame_size = (self.format.sample_rate / 50) as usize; // 960 samples per channel for 20ms
        let mut output = vec![0i16; frame_size * self.format.channels as usize];

        // Opus decode() returns the number of samples per channel decoded
        let samples_per_channel = self
            .decoder
            .decode(data, &mut output, fec)
            .map_err(|e| AudioBridgeError::OpusError(format!("Decode failed: {:?}", e)))?;

        // Total interleaved samples = samples_per_channel * channels
        let total_samples = samples_per_channel * self.format.channels as usize;
        output.truncate(total_samples);
        Ok(output)
    }

    /// Decode packet loss concealment (when packet is lost)
    pub fn decode_plc(&mut self) -> Result<Vec<i16>> {
        let frame_size = (self.format.sample_rate / 50) as usize;
        let output_size = frame_size * self.format.channels as usize;
        let mut output = vec![0i16; output_size];

        // Pass empty slice to trigger PLC
        let len = self
            .decoder
            .decode(&[], &mut output, false)
            .map_err(|e| AudioBridgeError::OpusError(format!("PLC decode failed: {:?}", e)))?;

        output.truncate(len * self.format.channels as usize);
        Ok(output)
    }

    pub fn format(&self) -> AudioFormat {
        self.format
    }
}
