use audio_bridge_core::rtp::{RtpHeader, RtpPacket};
use audio_bridge_core::{transport::RtpSender, Result};
use std::net::SocketAddr;

#[tokio::test]
async fn test_sender_creation() -> Result<()> {
    let target: SocketAddr = "127.0.0.1:5004".parse().unwrap();
    let sender = RtpSender::new(target).await?;

    // Verify sender is bound to some ephemeral port
    assert!(sender.local_addr()?.port() > 0);

    Ok(())
}

#[tokio::test]
async fn test_send_packet() -> Result<()> {
    let target: SocketAddr = "127.0.0.1:5004".parse().unwrap();
    let mut sender = RtpSender::new(target).await?;

    let header = RtpHeader {
        version: 2,
        padding: false,
        extension: false,
        csrc_count: 0,
        marker: false,
        payload_type: 96,
        sequence_number: 1,
        timestamp: 960,
        ssrc: 0x12345678,
    };

    let packet = RtpPacket::new(header, vec![0u8; 100]);
    let bytes_sent = sender.send(&packet).await?;

    assert_eq!(bytes_sent, 112); // 12-byte header + 100 payload

    Ok(())
}
