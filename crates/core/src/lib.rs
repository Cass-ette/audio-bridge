//! # Audio Bridge Core Library
//!
//! Low-latency audio transmission primitives for streaming audio over networks.
//!
//! ## Features
//!
//! - **Opus Codec**: High-quality audio encoding/decoding (128kbps VBR)
//! - **RTP Protocol**: Real-time Transport Protocol for packet framing
//! - **Jitter Buffer**: Packet reordering and loss detection
//!
//! ## Example Usage
//!
//! ### Basic Encoding and Decoding
//!
//! ```rust
//! use audio_bridge_core::codec::{AudioFormat, OpusEncoder, OpusDecoder};
//!
//! # fn main() -> audio_bridge_core::Result<()> {
//! let format = AudioFormat {
//!     sample_rate: 48000,
//!     channels: 2,
//!     bitrate: 128000,
//! };
//!
//! let mut encoder = OpusEncoder::new(format)?;
//! let mut decoder = OpusDecoder::new(format)?;
//!
//! // Encode 20ms of audio (960 samples * 2 channels)
//! let pcm = vec![0i16; 1920];
//! let encoded = encoder.encode(&pcm)?;
//!
//! // Decode back to PCM
//! let decoded = decoder.decode(&encoded, false)?;
//! assert_eq!(decoded.len(), 1920);
//! # Ok(())
//! # }
//! ```
//!
//! See `examples/simple_encoder.rs` for a complete working example.

pub mod codec;
pub mod rtp;
pub mod buffer;
pub mod error;

pub use error::{Result, AudioBridgeError};
