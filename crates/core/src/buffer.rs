use crate::rtp::RtpPacket;
use std::collections::VecDeque;

/// Statistics about jitter buffer performance
#[derive(Debug, Clone, Copy, Default)]
pub struct JitterBufferStats {
    pub packets_received: u64,
    pub packets_lost: u64,
    pub buffer_size: usize,
    pub last_sequence: Option<u16>,
}

/// Jitter buffer for reordering and smoothing RTP packets
pub struct JitterBuffer {
    packets: VecDeque<RtpPacket>,
    max_size: usize,
    stats: JitterBufferStats,
}

impl JitterBuffer {
    /// Create new jitter buffer with maximum capacity
    pub fn new(max_size: usize) -> Self {
        Self {
            packets: VecDeque::with_capacity(max_size),
            max_size,
            stats: JitterBufferStats::default(),
        }
    }

    /// Insert packet into buffer (maintains sorted order by sequence number)
    pub fn insert(&mut self, packet: RtpPacket) {
        let seq = packet.header.sequence_number;

        // Update packet loss statistics based on highest sequence seen
        // Note: This approach detects gaps but may have false positives with
        // out-of-order delivery. For accurate loss detection, track expected sequence
        // and only count loss when packets are consumed (popped) from buffer.
        if let Some(last_seq) = self.stats.last_sequence {
            // Only update last_sequence if this packet has a higher sequence number
            if sequence_greater_than(seq, last_seq) {
                let expected_seq = last_seq.wrapping_add(1);
                if seq != expected_seq {
                    // Calculate number of lost packets (handle wraparound)
                    let lost = if seq > expected_seq {
                        (seq - expected_seq) as u64
                    } else {
                        // Wraparound case: (65536 - expected) + seq
                        ((65536 - expected_seq as u32) + seq as u32) as u64
                    };
                    self.stats.packets_lost += lost;
                }
                self.stats.last_sequence = Some(seq);
            }
            // If seq <= last_seq, this is an out-of-order or duplicate packet
            // Don't update last_sequence, don't count as loss
        } else {
            // First packet
            self.stats.last_sequence = Some(seq);
        }

        self.stats.packets_received += 1;

        // Find insertion position - insert before the first packet with larger sequence number
        let pos = self.packets
            .iter()
            .position(|p| sequence_greater_than(p.header.sequence_number, seq))
            .unwrap_or(self.packets.len());

        self.packets.insert(pos, packet);

        // Enforce capacity limit by dropping oldest packets
        while self.packets.len() > self.max_size {
            self.packets.pop_front();
        }

        self.stats.buffer_size = self.packets.len();
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

    /// Get current statistics
    pub fn stats(&self) -> JitterBufferStats {
        self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = JitterBufferStats::default();
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
