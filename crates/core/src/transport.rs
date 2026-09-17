//! Network transmission layer for RTP packets over UDP

use crate::Result;
use crate::rtp::RtpPacket;
use std::net::SocketAddr;
use tokio::net::UdpSocket;

/// RTP packet sender over UDP
pub struct RtpSender {
    socket: UdpSocket,
    target: SocketAddr,
}

impl RtpSender {
    /// Create new sender bound to ephemeral port, sending to target
    pub async fn new(target: SocketAddr) -> Result<Self> {
        let local_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
        let socket = UdpSocket::bind(local_addr).await?;

        Ok(Self { socket, target })
    }

    /// Get local address of bound socket
    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.socket.local_addr()?)
    }

    /// Send RTP packet to target address
    ///
    /// Returns number of bytes sent (should equal packet size)
    pub async fn send(&self, packet: &RtpPacket) -> Result<usize> {
        let bytes = packet.to_bytes();
        let sent = self.socket.send_to(&bytes, self.target).await?;
        Ok(sent)
    }
}
