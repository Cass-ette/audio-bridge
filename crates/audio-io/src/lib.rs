//! Audio Bridge I/O
//!
//! Platform-specific audio capture and playback implementations.

pub mod config;
pub mod error;
pub mod mock;
pub mod receiver;
pub mod sender;
pub mod traits;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

pub use config::{Bitrate, ReceiverConfig, SenderConfig};
pub use error::{AudioIoError, Result};
pub use receiver::{AudioReceiver, ReceiverStats};
pub use sender::{AudioSender, SenderStats};
