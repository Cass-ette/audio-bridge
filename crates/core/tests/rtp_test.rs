use audio_bridge_core::rtp::RtpHeader;

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
    assert_eq!(bytes[1], 96);   // Payload type 96
}

#[test]
fn test_rtp_header_deserialize() {
    let bytes = vec![
        0x80, 96,       // V=2, PT=96
        0x04, 0xD2,     // Sequence number = 1234
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
