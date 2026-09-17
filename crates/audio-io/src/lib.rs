//! Audio Bridge I/O
//!
//! Platform-specific audio capture and playback implementations.

pub mod traits;
pub mod error;
pub mod config;

pub use error::{AudioIoError, Result};
pub use config::{SenderConfig, ReceiverConfig, Bitrate};
