//! Simple loopback example: send RTP packets to self and receive them
//!
//! Run with: cargo run --example simple_loopback -p audio-bridge-core

use audio_bridge_core::rtp::{RtpHeader, RtpPacket};
use audio_bridge_core::transport::{RtpReceiver, RtpSender};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> audio_bridge_core::Result<()> {
    // Create receiver first to get its address
    let receiver_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let mut receiver = RtpReceiver::new(receiver_addr).await?;
    let receiver_addr = receiver.local_addr()?;

    println!("Receiver listening on {}", receiver_addr);

    // Create sender targeting the receiver
    let mut sender = RtpSender::new(receiver_addr).await?;
    println!("Sender bound to {}", sender.local_addr()?);

    // Send 5 packets
    for i in 0..5 {
        let header = RtpHeader {
            version: 2,
            padding: false,
            extension: false,
            csrc_count: 0,
            marker: false,
            payload_type: 96, // Opus dynamic payload type
            sequence_number: i,
            timestamp: i as u32 * 960, // 20ms at 48kHz
            ssrc: 0x12345678,
        };
        let packet = RtpPacket::new(header, vec![0xAA; 100]); // 100 bytes of dummy audio

        sender.send(&packet).await?;
        println!("Sent packet {}: seq={}, ts={}", i, packet.header.sequence_number, packet.header.timestamp);
    }

    // Receive and print packets
    println!("\nReceiving packets...");
    for _ in 0..5 {
        let packet = receiver.receive().await?;
        println!("Received packet: seq={}, ts={}, payload_len={}",
                 packet.header.sequence_number, packet.header.timestamp, packet.payload.len());
    }

    // Print final stats
    let sender_stats = sender.stats();
    let receiver_stats = receiver.stats();

    println!("\n=== Statistics ===");
    println!("Sender:   {} packets, {} bytes",
             sender_stats.packets_sent, sender_stats.bytes_sent);
    println!("Receiver: {} packets, {} bytes",
             receiver_stats.packets_received, receiver_stats.bytes_received);

    Ok(())
}
