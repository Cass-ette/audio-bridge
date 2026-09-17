//! Audio Bridge I/O
//!
//! Platform-specific audio capture and playback implementations.

pub mod traits;
pub mod error;
pub mod config;
pub mod mock;
pub mod sender;
pub mod receiver;

pub use error::{AudioIoError, Result};
pub use config::{SenderConfig, ReceiverConfig, Bitrate};
pub use sender::{AudioSender, SenderStats};
pub use receiver::{AudioReceiver, ReceiverStats};
