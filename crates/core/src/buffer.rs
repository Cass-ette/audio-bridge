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
    packets_received: u64,
    packets_lost: u64,
    last_sequence: Option<u16>,
}

impl JitterBuffer {
    /// Create new jitter buffer with maximum capacity
    pub fn new(max_size: usize) -> Self {
        Self {
            packets: VecDeque::with_capacity(max_size),
            max_size,
            packets_received: 0,
            packets_lost: 0,
            last_sequence: None,
        }
    }

    /// Insert packet into buffer (maintains sorted order by sequence number)
    pub fn insert(&mut self, packet: RtpPacket) {
        let seq = packet.header.sequence_number;

        // Track packet loss by detecting gaps in sequence numbers.
        // Only updates last_sequence when a higher sequence number arrives,
        // so out-of-order packets don't trigger false positives.
        if let Some(last_seq) = self.last_sequence {
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
                    self.packets_lost += lost;
                }
                self.last_sequence = Some(seq);
            }
            // If seq <= last_seq, this is an out-of-order or duplicate packet
            // Don't update last_sequence, don't count as loss
        } else {
            // First packet
            self.last_sequence = Some(seq);
        }

        self.packets_received += 1;

        // Find insertion position - insert before the first packet with larger sequence number
        let pos = self
            .packets
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

    /// Get current statistics
    pub fn stats(&self) -> JitterBufferStats {
        JitterBufferStats {
            packets_received: self.packets_received,
            packets_lost: self.packets_lost,
            buffer_size: self.packets.len(), // Computed on-demand to stay accurate
            last_sequence: self.last_sequence,
        }
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.packets_received = 0;
        self.packets_lost = 0;
        self.last_sequence = None;
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
