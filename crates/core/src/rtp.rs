use crate::{AudioBridgeError, Result};

/// RTP header according to RFC 3550
#[derive(Debug, Clone, PartialEq)]
pub struct RtpHeader {
    pub version: u8,           // Always 2
    pub padding: bool,
    pub extension: bool,
    pub csrc_count: u8,
    pub marker: bool,
    pub payload_type: u8,      // 96 for Opus (dynamic)
    pub sequence_number: u16,
    pub timestamp: u32,        // Sampling instant
    pub ssrc: u32,             // Synchronization source identifier
}

impl RtpHeader {
    /// Serialize header to 12 bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(12);

        // Byte 0: V(2), P(1), X(1), CC(4)
        let byte0 = (self.version << 6)
            | ((self.padding as u8) << 5)
            | ((self.extension as u8) << 4)
            | (self.csrc_count & 0x0F);
        bytes.push(byte0);

        // Byte 1: M(1), PT(7)
        let byte1 = ((self.marker as u8) << 7) | (self.payload_type & 0x7F);
        bytes.push(byte1);

        // Bytes 2-3: Sequence number
        bytes.extend_from_slice(&self.sequence_number.to_be_bytes());

        // Bytes 4-7: Timestamp
        bytes.extend_from_slice(&self.timestamp.to_be_bytes());

        // Bytes 8-11: SSRC
        bytes.extend_from_slice(&self.ssrc.to_be_bytes());

        bytes
    }

    /// Deserialize header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 12 {
            return Err(AudioBridgeError::RtpError(
                format!("RTP header too short: {} bytes", bytes.len())
            ));
        }

        let version = (bytes[0] >> 6) & 0x03;
        if version != 2 {
            return Err(AudioBridgeError::RtpError(
                format!("Unsupported RTP version: {}", version)
            ));
        }

        Ok(RtpHeader {
            version,
            padding: (bytes[0] & 0x20) != 0,
            extension: (bytes[0] & 0x10) != 0,
            csrc_count: bytes[0] & 0x0F,
            marker: (bytes[1] & 0x80) != 0,
            payload_type: bytes[1] & 0x7F,
            sequence_number: u16::from_be_bytes([bytes[2], bytes[3]]),
            timestamp: u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            ssrc: u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        })
    }
}

/// RTP packet placeholder for future implementation
#[derive(Debug, Clone)]
pub struct RtpPacket {
    pub header: RtpHeader,
    pub payload: Vec<u8>,
}
