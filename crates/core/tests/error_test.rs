use audio_bridge_core::{AudioBridgeError, Result};

#[test]
fn test_error_display() {
    let err = AudioBridgeError::OpusError("test error".to_string());
    assert_eq!(err.to_string(), "Opus codec error: test error");
}

#[test]
fn test_result_type() {
    let ok: Result<i32> = Ok(42);
    assert_eq!(ok.unwrap(), 42);

    let err: Result<i32> = Err(AudioBridgeError::RtpError("failed".to_string()));
    assert!(err.is_err());
}

#[test]
fn test_io_error_conversion() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let bridge_err: AudioBridgeError = io_err.into();
    assert!(matches!(bridge_err, AudioBridgeError::Io(_)));
}

#[test]
fn test_packet_validation_error() {
    let err = AudioBridgeError::PacketValidation("invalid header".to_string());
    assert_eq!(
        err.to_string(),
        "RTP packet validation failed: invalid header"
    );
}
