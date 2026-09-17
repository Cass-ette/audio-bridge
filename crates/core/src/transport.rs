//! Network transmission layer for RTP packets over UDP

use crate::{AudioBridgeError, Result};
use std::net::SocketAddr;
use tokio::net::UdpSocket;

// Module will contain RtpSender and RtpReceiver
