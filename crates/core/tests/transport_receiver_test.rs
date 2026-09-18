use audio_bridge_core::{
    rtp::{RtpHeader, RtpPacket},
    transport::RtpReceiver,
    transport::RtpSender,
    Result,
};
use std::net::SocketAddr;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_receiver_creation() -> Result<()> {
    let bind_addr: SocketAddr = "127.0.0.1:5004".parse().unwrap();
    let receiver = RtpReceiver::new(bind_addr).await?;

    assert_eq!(receiver.local_addr()?.port(), 5004);

    Ok(())
}

#[tokio::test]
async fn test_receive_packet() -> Result<()> {
    // Start receiver first
    let receiver_addr: SocketAddr = "127.0.0.1:15004".parse().unwrap();
    let mut receiver = RtpReceiver::new(receiver_addr).await?;

    // Create sender targeting receiver
    let mut sender = RtpSender::new(receiver_addr).await?;

    // Send packet
    let header = RtpHeader {
        version: 2,
        padding: false,
        extension: false,
        csrc_count: 0,
        marker: false,
        payload_type: 96,
        sequence_number: 42,
        timestamp: 1920,
        ssrc: 0xABCDEF00,
    };
    let payload = vec![1, 2, 3, 4, 5];
    let packet = RtpPacket::new(header, payload.clone());

    sender.send(&packet).await?;

    // Receive with timeout
    let received = timeout(Duration::from_millis(100), receiver.receive())
        .await
        .expect("receive timeout")?;

    assert_eq!(received.header.sequence_number, 42);
    assert_eq!(received.header.timestamp, 1920);
    assert_eq!(received.header.ssrc, 0xABCDEF00);
    assert_eq!(received.payload, payload);

    Ok(())
}
