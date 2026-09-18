//! End-to-end integration test: Opus encode → RTP send → RTP receive → Opus decode
//!
//! Tests the full pipeline:
//! 1. Generate PCM audio (sine wave)
//! 2. Encode with OpusEncoder
//! 3. Wrap in RTP packet and send via RtpSender
//! 4. Receive via RtpReceiver (with timeout)
//! 5. Decode with OpusDecoder
//! 6. Verify audio similarity and stats

use audio_bridge_core::{
    codec::{AudioFormat, OpusDecoder, OpusEncoder},
    rtp::{RtpHeader, RtpPacket},
    transport::{RtpReceiver, RtpSender},
};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::timeout;

/// Generate stereo sine wave: 960 samples per channel (20ms at 48kHz)
fn generate_sine_wave() -> Vec<i16> {
    let sample_rate = 48000;
    let duration_ms = 20;
    let frequency = 440.0; // A4 note
    let samples_per_channel = (sample_rate * duration_ms) / 1000; // 960

    let mut pcm = Vec::with_capacity(samples_per_channel * 2);
    for i in 0..samples_per_channel {
        let t = i as f64 / sample_rate as f64;
        let sample = (2.0 * std::f64::consts::PI * frequency * t).sin();
        let value = (sample * 16000.0) as i16;

        // Stereo: left and right channels
        pcm.push(value);
        pcm.push(value);
    }

    pcm
}

#[tokio::test]
async fn test_end_to_end_loopback() {
    // 1. Setup: codec configuration
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 2,
        bitrate: 128000,
    };

    let mut encoder = OpusEncoder::new(format).expect("Failed to create encoder");
    let mut decoder = OpusDecoder::new(format).expect("Failed to create decoder");

    // 2. Setup: network transport (loopback)
    let receiver_addr: SocketAddr = "127.0.0.1:45004".parse().unwrap();
    let mut receiver = RtpReceiver::new(receiver_addr)
        .await
        .expect("Failed to create receiver");

    let mut sender = RtpSender::new(receiver_addr)
        .await
        .expect("Failed to create sender");

    // 3. Generate and encode audio
    let original_pcm = generate_sine_wave();
    assert_eq!(original_pcm.len(), 1920); // 960 samples × 2 channels

    let opus_payload = encoder
        .encode(&original_pcm)
        .expect("Failed to encode audio");

    println!(
        "Encoded {} PCM samples → {} Opus bytes",
        original_pcm.len(),
        opus_payload.len()
    );

    // 4. Wrap in RTP packet and send
    let rtp_header = RtpHeader {
        version: 2,
        padding: false,
        extension: false,
        csrc_count: 0,
        marker: false,
        payload_type: 96, // Dynamic payload type for Opus
        sequence_number: 1000,
        timestamp: 480000, // Arbitrary starting timestamp
        ssrc: 0x12345678,
    };

    let packet = RtpPacket::new(rtp_header.clone(), opus_payload.clone());
    let sent_bytes = sender.send(&packet).await.expect("Failed to send packet");
    println!("Sent {} bytes via UDP", sent_bytes);

    // 5. Receive with timeout
    let received_packet = timeout(Duration::from_millis(100), receiver.receive())
        .await
        .expect("Receive timeout")
        .expect("Failed to receive packet");

    println!(
        "Received packet with {} byte payload",
        received_packet.payload.len()
    );

    // 6. Verify RTP header fields
    assert_eq!(received_packet.header.version, 2);
    assert_eq!(received_packet.header.payload_type, 96);
    assert_eq!(received_packet.header.sequence_number, 1000);
    assert_eq!(received_packet.header.timestamp, 480000);
    assert_eq!(received_packet.header.ssrc, 0x12345678);

    // 7. Decode audio
    let decoded_pcm = decoder
        .decode(&received_packet.payload, false)
        .expect("Failed to decode audio");

    assert_eq!(decoded_pcm.len(), 1920); // Same size as input

    // 8. Verify audio similarity (lossy codec, expect close but not exact match)
    // Important: Opus has algorithmic delay (lookahead) of ~312 samples per channel
    // For stereo, that's 624 total samples. We need to compare:
    // decoded[lookahead..] with original[0..len-lookahead]
    let lookahead_offset = 624; // 312 samples per channel * 2 channels
    let tolerance = 5000; // Allow up to ±5000 difference (lossy codec + network)

    let mut similar_samples = 0;
    let comparison_length = original_pcm.len() - lookahead_offset;

    for (orig, decoded) in original_pcm
        .iter()
        .take(comparison_length)
        .zip(decoded_pcm.iter().skip(lookahead_offset))
    {
        if (orig - decoded).abs() <= tolerance {
            similar_samples += 1;
        }
    }

    let similarity_ratio = similar_samples as f64 / comparison_length as f64;
    println!(
        "Audio similarity: {}/{} samples within ±{} ({:.1}%)",
        similar_samples,
        comparison_length,
        tolerance,
        similarity_ratio * 100.0
    );

    // Expect at least 90% similarity after accounting for lookahead
    let min_similar = (comparison_length as f64 * 0.9) as usize;
    assert!(
        similar_samples >= min_similar,
        "Expected at least {}/{} samples similar, got {}",
        min_similar,
        comparison_length,
        similar_samples
    );

    // 9. Verify transport statistics
    let sender_stats = sender.stats();
    let receiver_stats = receiver.stats();

    assert_eq!(sender_stats.packets_sent, 1);
    assert_eq!(receiver_stats.packets_received, 1);
    assert!(sender_stats.bytes_sent > 0);
    assert!(receiver_stats.bytes_received > 0);

    println!("✓ End-to-end loopback test passed");
    println!(
        "  Sender: {} packets, {} bytes",
        sender_stats.packets_sent, sender_stats.bytes_sent
    );
    println!(
        "  Receiver: {} packets, {} bytes",
        receiver_stats.packets_received, receiver_stats.bytes_received
    );
}
