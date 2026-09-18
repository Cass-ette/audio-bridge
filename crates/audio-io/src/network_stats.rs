//! Network quality monitoring and statistics

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Network quality metrics
#[derive(Debug, Clone)]
pub struct NetworkStats {
    /// Packets received
    pub packets_received: u64,
    /// Packets lost (detected by sequence gaps)
    pub packets_lost: u64,
    /// Out-of-order packets
    pub packets_reordered: u64,
    /// Average network jitter (ms)
    pub jitter_ms: f64,
    /// Current round-trip time estimate (ms)
    pub rtt_ms: f64,
    /// Packet loss rate (0.0 - 1.0)
    pub loss_rate: f64,
    /// Receive bitrate (kbps)
    pub bitrate_kbps: f64,
    /// Buffer health (0-100, 100 = healthy)
    pub buffer_health: u8,
}

impl Default for NetworkStats {
    fn default() -> Self {
        Self {
            packets_received: 0,
            packets_lost: 0,
            packets_reordered: 0,
            jitter_ms: 0.0,
            rtt_ms: 0.0,
            loss_rate: 0.0,
            bitrate_kbps: 0.0,
            buffer_health: 100,
        }
    }
}

/// Network quality monitor
pub struct NetworkMonitor {
    stats: Arc<Mutex<NetworkStats>>,
    arrival_times: Arc<Mutex<VecDeque<(u16, Instant)>>>, // (seq, arrival_time)
    last_seq: Arc<Mutex<Option<u16>>>,
    bytes_received: Arc<Mutex<VecDeque<(Instant, usize)>>>, // For bitrate calculation
}

impl NetworkMonitor {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(Mutex::new(NetworkStats::default())),
            arrival_times: Arc::new(Mutex::new(VecDeque::with_capacity(100))),
            last_seq: Arc::new(Mutex::new(None)),
            bytes_received: Arc::new(Mutex::new(VecDeque::with_capacity(100))),
        }
    }

    /// Record a received packet
    pub fn record_packet(&self, seq: u16, payload_size: usize) {
        let now = Instant::now();

        let mut stats = self.stats.lock().unwrap();
        stats.packets_received += 1;

        // Check for loss and reordering
        let mut last_seq = self.last_seq.lock().unwrap();
        if let Some(prev_seq) = *last_seq {
            let expected_seq = prev_seq.wrapping_add(1);
            if seq != expected_seq {
                let gap = seq.wrapping_sub(expected_seq);
                if gap < 1000 {
                    // Forward gap = loss
                    stats.packets_lost += gap as u64;
                } else {
                    // Backward gap = reordering
                    stats.packets_reordered += 1;
                }
            }
        }
        *last_seq = Some(seq);

        // Record arrival time for jitter calculation
        let mut arrivals = self.arrival_times.lock().unwrap();
        arrivals.push_back((seq, now));
        if arrivals.len() > 100 {
            arrivals.pop_front();
        }

        // Calculate jitter (RFC 3550 style - inter-arrival variance)
        if arrivals.len() >= 2 {
            let mut diffs = Vec::new();
            for window in arrivals.iter().collect::<Vec<_>>().windows(2) {
                let (seq1, t1) = window[0];
                let (seq2, t2) = window[1];
                let time_diff = t2.duration_since(*t1).as_secs_f64() * 1000.0; // ms
                let seq_diff = seq2.wrapping_sub(*seq1) as f64;
                if seq_diff > 0.0 && seq_diff < 100.0 {
                    let expected_time = seq_diff * 20.0; // 20ms per packet
                    diffs.push((time_diff - expected_time).abs());
                }
            }
            if !diffs.is_empty() {
                stats.jitter_ms = diffs.iter().sum::<f64>() / diffs.len() as f64;
            }
        }

        // Record bytes for bitrate
        let mut bytes = self.bytes_received.lock().unwrap();
        bytes.push_back((now, payload_size));

        // Remove old entries (older than 1 second)
        while let Some((time, _)) = bytes.front() {
            if now.duration_since(*time) > Duration::from_secs(1) {
                bytes.pop_front();
            } else {
                break;
            }
        }

        // Calculate bitrate
        let total_bytes: usize = bytes.iter().map(|(_, size)| size).sum();
        stats.bitrate_kbps = (total_bytes as f64 * 8.0) / 1000.0;

        // Calculate loss rate
        let total_packets = stats.packets_received + stats.packets_lost;
        if total_packets > 0 {
            stats.loss_rate = stats.packets_lost as f64 / total_packets as f64;
        }

        drop(stats);
    }

    /// Update buffer health (0-100)
    pub fn update_buffer_health(&self, queue_size: usize, optimal_size: usize) {
        let mut stats = self.stats.lock().unwrap();

        // Health score based on distance from optimal
        // Optimal = 100, empty or full = 0
        let health = if queue_size == 0 {
            0
        } else if queue_size >= optimal_size * 2 {
            20 // Too full
        } else {
            let distance = (queue_size as i32 - optimal_size as i32).abs();
            let normalized = 1.0 - (distance as f64 / optimal_size as f64).min(1.0);
            (normalized * 100.0) as u8
        };

        stats.buffer_health = health;
    }

    /// Get current statistics
    pub fn get_stats(&self) -> NetworkStats {
        self.stats.lock().unwrap().clone()
    }

    /// Get network quality grade (A-F)
    pub fn get_quality_grade(&self) -> char {
        let stats = self.get_stats();

        // Scoring based on multiple factors
        let mut score = 100.0;

        // Loss rate penalty (each 1% loss = -10 points)
        score -= stats.loss_rate * 1000.0;

        // Jitter penalty (each 10ms = -5 points)
        score -= (stats.jitter_ms / 10.0) * 5.0;

        // Buffer health penalty
        score -= (100 - stats.buffer_health as i32) as f64 * 0.3;

        // Convert to grade
        if score >= 90.0 { 'A' }
        else if score >= 80.0 { 'B' }
        else if score >= 70.0 { 'C' }
        else if score >= 60.0 { 'D' }
        else { 'F' }
    }

    /// Check if network quality is acceptable
    pub fn is_quality_acceptable(&self) -> bool {
        let stats = self.get_stats();
        stats.loss_rate < 0.05  // < 5% loss
            && stats.jitter_ms < 50.0  // < 50ms jitter
            && stats.buffer_health > 30  // Buffer not critically low/high
    }
}

impl Default for NetworkMonitor {
    fn default() -> Self {
        Self::new()
    }
}
