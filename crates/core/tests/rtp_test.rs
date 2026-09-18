use audio_bridge_core::rtp::{RtpHeader, RtpPacket};

#[test]
fn test_rtp_header_serialize() {
    let header = RtpHeader {
        version: 2,
        padding: false,
        extension: false,
        csrc_count: 0,
        marker: false,
        payload_type: 96, // Dynamic payload type for Opus
        sequence_number: 1234,
        timestamp: 48000,
        ssrc: 0x12345678,
    };

    let bytes = header.to_bytes();
    assert_eq!(bytes.len(), 12); // RTP header is always 12 bytes
    assert_eq!(bytes[0], 0x80); // Version 2, no padding, no extension, no CSRC
    assert_eq!(bytes[1], 96); // Payload type 96
}

#[test]
fn test_rtp_header_deserialize() {
    let bytes = vec![
        0x80, 96, // V=2, PT=96
        0x04, 0xD2, // Sequence number = 1234
        0x00, 0x00, 0xBB, 0x80, // Timestamp = 48000
        0x12, 0x34, 0x56, 0x78, // SSRC
    ];

    let header = RtpHeader::from_bytes(&bytes).unwrap();
    assert_eq!(header.version, 2);
    assert_eq!(header.payload_type, 96);
    assert_eq!(header.sequence_number, 1234);
    assert_eq!(header.timestamp, 48000);
    assert_eq!(header.ssrc, 0x12345678);
}

#[test]
fn test_rtp_packet_roundtrip() {
    let header = RtpHeader {
        version: 2,
        padding: false,
        extension: false,
        csrc_count: 0,
        marker: false,
        payload_type: 96,
        sequence_number: 100,
        timestamp: 960,
        ssrc: 0xABCDEF01,
    };

    let payload = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let packet = RtpPacket::new(header.clone(), payload.clone());

    let bytes = packet.to_bytes();
    let decoded = RtpPacket::from_bytes(&bytes).unwrap();

    assert_eq!(decoded.header, header);
    assert_eq!(decoded.payload, payload);
}

#[test]
fn test_rtp_packet_increment_sequence() {
    let mut packet = RtpPacket::new(
        RtpHeader {
            version: 2,
            padding: false,
            extension: false,
            csrc_count: 0,
            marker: false,
            payload_type: 96,
            sequence_number: 65535,
            timestamp: 0,
            ssrc: 0,
        },
        vec![],
    );

    packet.increment_sequence();
    assert_eq!(packet.header.sequence_number, 0); // Wrap around
}
