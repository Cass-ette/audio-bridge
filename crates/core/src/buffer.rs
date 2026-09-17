use crate::rtp::RtpPacket;
use std::collections::VecDeque;

/// Jitter buffer for reordering and smoothing RTP packets
pub struct JitterBuffer {
    packets: VecDeque<RtpPacket>,
    max_size: usize,
}

impl JitterBuffer {
    /// Create new jitter buffer with maximum capacity
    pub fn new(max_size: usize) -> Self {
        Self {
            packets: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// Insert packet into buffer (maintains sorted order by sequence number)
    pub fn insert(&mut self, packet: RtpPacket) {
        // Find insertion position - insert before the first packet with larger sequence number
        let seq = packet.header.sequence_number;
        let pos = self.packets
            .iter()
            .position(|p| sequence_greater_than(p.header.sequence_number, seq))
            .unwrap_or(self.packets.len());

        self.packets.insert(pos, packet);

        // Enforce capacity limit by dropping oldest packets
        while self.packets.len() > self.max_size {
            self.packets.pop_front();
        }
    }

    /// Remove and return the oldest packet (by sequence number)
    pub fn pop(&mut self) -> Option<RtpPacket> {
        self.packets.pop_front()
    }

    /// Peek at the oldest packet without removing it
    pub fn peek(&self) -> Option<&RtpPacket> {
        self.packets.front()
    }

    /// Number of packets in buffer
    pub fn len(&self) -> usize {
        self.packets.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.packets.is_empty()
    }

    /// Clear all packets
    pub fn clear(&mut self) {
        self.packets.clear();
    }
}

/// Compare sequence numbers handling wraparound
/// Returns true if `a` is greater than `b` (considering wraparound)
///
/// Uses the "half the sequence number space" rule: if the difference between
/// two sequence numbers is > 32768 (half of u16::MAX), assume wraparound occurred.
/// This works because RTP streams rarely have more than 32768 packets in flight.
fn sequence_greater_than(a: u16, b: u16) -> bool {
    // Handle wraparound: if difference is > 32768, assume wraparound occurred
    let diff = a.wrapping_sub(b);
    diff > 0 && diff < 32768
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequence_greater_than() {
        assert!(sequence_greater_than(100, 99));
        assert!(!sequence_greater_than(99, 100));

        // Wraparound cases
        assert!(sequence_greater_than(1, 65535)); // 1 > 65535 (wraparound)
        assert!(!sequence_greater_than(65535, 1)); // 65535 < 1 (wraparound)

        // Edge case
        assert!(!sequence_greater_than(100, 100)); // Equal
    }
}
