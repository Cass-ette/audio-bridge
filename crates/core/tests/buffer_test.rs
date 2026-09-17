use audio_bridge_core::buffer::JitterBuffer;
use audio_bridge_core::rtp::{RtpHeader, RtpPacket};

fn create_test_packet(seq: u16, timestamp: u32, data: Vec<u8>) -> RtpPacket {
    RtpPacket::new(
        RtpHeader {
            version: 2,
            padding: false,
            extension: false,
            csrc_count: 0,
            marker: false,
            payload_type: 96,
            sequence_number: seq,
            timestamp,
            ssrc: 0x12345678,
        },
        data,
    )
}

#[test]
fn test_jitter_buffer_insert_ordered() {
    let mut buffer = JitterBuffer::new(5);

    let pkt1 = create_test_packet(1, 960, vec![1, 2, 3]);
    let pkt2 = create_test_packet(2, 1920, vec![4, 5, 6]);

    buffer.insert(pkt1);
    buffer.insert(pkt2);

    assert_eq!(buffer.len(), 2);
}

#[test]
fn test_jitter_buffer_reorder() {
    let mut buffer = JitterBuffer::new(5);

    // Insert out of order
    let pkt2 = create_test_packet(2, 1920, vec![4, 5, 6]);
    let pkt1 = create_test_packet(1, 960, vec![1, 2, 3]);
    let pkt3 = create_test_packet(3, 2880, vec![7, 8, 9]);

    buffer.insert(pkt2);
    buffer.insert(pkt1);
    buffer.insert(pkt3);

    // Should be sorted by sequence number
    let retrieved1 = buffer.pop().unwrap();
    let retrieved2 = buffer.pop().unwrap();
    let retrieved3 = buffer.pop().unwrap();

    assert_eq!(retrieved1.header.sequence_number, 1);
    assert_eq!(retrieved2.header.sequence_number, 2);
    assert_eq!(retrieved3.header.sequence_number, 3);
}

#[test]
fn test_jitter_buffer_capacity() {
    let mut buffer = JitterBuffer::new(3);

    buffer.insert(create_test_packet(1, 960, vec![1]));
    buffer.insert(create_test_packet(2, 1920, vec![2]));
    buffer.insert(create_test_packet(3, 2880, vec![3]));
    buffer.insert(create_test_packet(4, 3840, vec![4])); // Over capacity

    // Oldest packet should be dropped
    assert_eq!(buffer.len(), 3);
    let first = buffer.pop().unwrap();
    assert_eq!(first.header.sequence_number, 2); // 1 was dropped
}
