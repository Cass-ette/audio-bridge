//! Statistics tracking tests for RtpSender and RtpReceiver

use audio_bridge_core::rtp::{RtpHeader, RtpPacket};
use audio_bridge_core::transport::{RtpReceiver, RtpSender};
use std::net::SocketAddr;

#[tokio::test]
async fn test_sender_stats() {
    let target: SocketAddr = "127.0.0.1:5004".parse().unwrap();
    let mut sender = RtpSender::new(target).await.unwrap();

    // Initial stats should be zero
    let stats = sender.stats();
    assert_eq!(stats.packets_sent, 0);
    assert_eq!(stats.bytes_sent, 0);

    // Send a packet
    let header = RtpHeader {
        version: 2,
        padding: false,
        extension: false,
        csrc_count: 0,
        marker: false,
        payload_type: 96,
        sequence_number: 1000,
        timestamp: 12345,
        ssrc: 0xDEADBEEF,
    };
    let packet = RtpPacket::new(header, vec![1, 2, 3, 4]);
    sender.send(&packet).await.unwrap();

    // Stats should increment
    let stats = sender.stats();
    assert_eq!(stats.packets_sent, 1);
    assert_eq!(stats.bytes_sent, packet.to_bytes().len() as u64);

    // Send another packet
    let header2 = RtpHeader {
        version: 2,
        padding: false,
        extension: false,
        csrc_count: 0,
        marker: false,
        payload_type: 96,
        sequence_number: 1001,
        timestamp: 12345,
        ssrc: 0xDEADBEEF,
    };
    let packet2 = RtpPacket::new(header2, vec![5, 6, 7, 8, 9]);
    sender.send(&packet2).await.unwrap();

    // Stats should accumulate
    let stats = sender.stats();
    assert_eq!(stats.packets_sent, 2);
    assert_eq!(
        stats.bytes_sent,
        (packet.to_bytes().len() + packet2.to_bytes().len()) as u64
    );
}

#[tokio::test]
async fn test_receiver_stats() {
    let bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let mut receiver = RtpReceiver::new(bind_addr).await.unwrap();
    let receiver_addr = receiver.local_addr().unwrap();

    // Initial stats should be zero
    let stats = receiver.stats();
    assert_eq!(stats.packets_received, 0);
    assert_eq!(stats.bytes_received, 0);

    // Send a packet to the receiver
    let mut sender = RtpSender::new(receiver_addr).await.unwrap();
    let header = RtpHeader {
        version: 2,
        padding: false,
        extension: false,
        csrc_count: 0,
        marker: false,
        payload_type: 96,
        sequence_number: 2000,
        timestamp: 54321,
        ssrc: 0xCAFEBABE,
    };
    let packet = RtpPacket::new(header, vec![10, 20, 30]);
    sender.send(&packet).await.unwrap();

    // Receive the packet
    let received = receiver.receive().await.unwrap();
    assert_eq!(received.payload, vec![10, 20, 30]);

    // Stats should increment
    let stats = receiver.stats();
    assert_eq!(stats.packets_received, 1);
    assert_eq!(stats.bytes_received, packet.to_bytes().len() as u64);

    // Send and receive another packet
    let header2 = RtpHeader {
        version: 2,
        padding: false,
        extension: false,
        csrc_count: 0,
        marker: false,
        payload_type: 96,
        sequence_number: 2001,
        timestamp: 54321,
        ssrc: 0xCAFEBABE,
    };
    let packet2 = RtpPacket::new(header2, vec![40, 50]);
    sender.send(&packet2).await.unwrap();
    let received2 = receiver.receive().await.unwrap();
    assert_eq!(received2.payload, vec![40, 50]);

    // Stats should accumulate
    let stats = receiver.stats();
    assert_eq!(stats.packets_received, 2);
    assert_eq!(
        stats.bytes_received,
        (packet.to_bytes().len() + packet2.to_bytes().len()) as u64
    );
}
