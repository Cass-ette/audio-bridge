//! Network transmission layer for RTP packets over UDP
//!
//! This module provides low-latency UDP transport for RTP packets, supporting both
//! sending and receiving operations with statistics tracking.
//!
//! # Features
//!
//! - **UDP-based RTP transmission**: Low-latency packet delivery using tokio async I/O
//! - **Statistics tracking**: Per-sender and per-receiver packet/byte counters
//! - **Simple API**: Async send/receive with automatic serialization
//! - **RFC 3550 compliant**: Works with standard RTP packets
//!
//! # Architecture
//!
//! The transport layer consists of two main components:
//!
//! - [`RtpSender`]: Sends RTP packets to a target address over UDP
//! - [`RtpReceiver`]: Receives RTP packets from any source on a bound port
//!
//! Both components track statistics ([`SenderStats`]/[`ReceiverStats`]) for monitoring
//! and debugging network performance.
//!
//! # Example
//!
//! ```no_run
//! use audio_bridge_core::rtp::{RtpHeader, RtpPacket};
//! use audio_bridge_core::transport::{RtpReceiver, RtpSender};
//! use std::net::SocketAddr;
//!
//! #[tokio::main]
//! async fn main() -> audio_bridge_core::Result<()> {
//!     // Create receiver
//!     let receiver_addr: SocketAddr = "127.0.0.1:5004".parse().unwrap();
//!     let mut receiver = RtpReceiver::new(receiver_addr).await?;
//!
//!     // Create sender targeting the receiver
//!     let mut sender = RtpSender::new(receiver_addr).await?;
//!
//!     // Send a packet
//!     let header = RtpHeader {
//!         version: 2,
//!         padding: false,
//!         extension: false,
//!         csrc_count: 0,
//!         marker: false,
//!         payload_type: 96,
//!         sequence_number: 0,
//!         timestamp: 0,
//!         ssrc: 0x12345678,
//!     };
//!     let packet = RtpPacket::new(header, vec![0; 100]);
//!     sender.send(&packet).await?;
//!
//!     // Receive the packet
//!     let received = receiver.receive().await?;
//!     println!("Received {} bytes", received.payload.len());
//!
//!     // Check statistics
//!     let stats = sender.stats();
//!     println!("Sent {} packets", stats.packets_sent);
//!
//!     Ok(())
//! }
//! ```
//!
//! For a complete working example, see `examples/simple_loopback.rs`.

use crate::Result;
use crate::rtp::RtpPacket;
use std::net::SocketAddr;
use tokio::net::UdpSocket;

/// Statistics for RTP sender
#[derive(Debug, Default, Clone, Copy)]
pub struct SenderStats {
    pub packets_sent: u64,
    pub bytes_sent: u64,
}

/// Statistics for RTP receiver
#[derive(Debug, Default, Clone, Copy)]
pub struct ReceiverStats {
    pub packets_received: u64,
    pub bytes_received: u64,
}

/// RTP packet sender over UDP
pub struct RtpSender {
    socket: UdpSocket,
    target: SocketAddr,
    stats: SenderStats,
}

impl RtpSender {
    /// Create new sender bound to ephemeral port, sending to target
    pub async fn new(target: SocketAddr) -> Result<Self> {
        let local_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
        let socket = UdpSocket::bind(local_addr).await?;

        Ok(Self {
            socket,
            target,
            stats: SenderStats::default(),
        })
    }

    /// Get local address of bound socket
    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.socket.local_addr()?)
    }

    /// Send RTP packet to target address
    ///
    /// Returns number of bytes sent (should equal packet size)
    pub async fn send(&mut self, packet: &RtpPacket) -> Result<usize> {
        let bytes = packet.to_bytes();
        let sent = self.socket.send_to(&bytes, self.target).await?;

        // Update statistics
        self.stats.packets_sent += 1;
        self.stats.bytes_sent += sent as u64;

        Ok(sent)
    }

    /// Get current sender statistics
    pub fn stats(&self) -> SenderStats {
        self.stats
    }
}

/// RTP packet receiver over UDP
pub struct RtpReceiver {
    socket: UdpSocket,
    stats: ReceiverStats,
}

impl RtpReceiver {
    /// Create new receiver listening on bind_addr
    ///
    /// Typically bind to 0.0.0.0:5004 for production, 127.0.0.1:5004 for testing
    pub async fn new(bind_addr: SocketAddr) -> Result<Self> {
        let socket = UdpSocket::bind(bind_addr).await?;
        Ok(Self {
            socket,
            stats: ReceiverStats::default(),
        })
    }

    /// Get local address of bound socket
    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.socket.local_addr()?)
    }

    /// Receive one RTP packet from socket
    ///
    /// Blocks until packet arrives. Buffer size is 2048 bytes (enough for max RTP packet).
    pub async fn receive(&mut self) -> Result<RtpPacket> {
        let mut buf = vec![0u8; 2048];
        let (len, _src_addr) = self.socket.recv_from(&mut buf).await?;

        buf.truncate(len);
        let packet = RtpPacket::from_bytes(&buf)?;

        // Update statistics
        self.stats.packets_received += 1;
        self.stats.bytes_received += len as u64;

        Ok(packet)
    }

    /// Get current receiver statistics
    pub fn stats(&self) -> ReceiverStats {
        self.stats
    }
}
