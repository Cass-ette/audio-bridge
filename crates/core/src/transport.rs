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

/// RTP packet receiver over UDP
pub struct RtpReceiver {
    socket: UdpSocket,
}

impl RtpReceiver {
    /// Create new receiver listening on bind_addr
    ///
    /// Typically bind to 0.0.0.0:5004 for production, 127.0.0.1:5004 for testing
    pub async fn new(bind_addr: SocketAddr) -> Result<Self> {
        let socket = UdpSocket::bind(bind_addr).await?;
        Ok(Self { socket })
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

        Ok(packet)
    }
}
