use audio_bridge_core::error::AudioBridgeError;
use audio_bridge_core::transport::RtpReceiver;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_bind_conflict() {
    // Bind first receiver to a random port
    let receiver1 = RtpReceiver::new("127.0.0.1:0".parse().unwrap())
        .await
        .expect("Failed to create first receiver");
    let addr = receiver1.local_addr().expect("Failed to get local address");

    // Try to bind second receiver to the same port - should fail
    let result = RtpReceiver::new(addr).await;

    assert!(result.is_err(), "Expected error when binding to occupied port");

    match result {
        Err(AudioBridgeError::Io(e)) => {
            // Verify it's an address-in-use error
            assert_eq!(e.kind(), std::io::ErrorKind::AddrInUse);
        }
        Err(e) => panic!("Expected Io error, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

#[tokio::test]
async fn test_invalid_rtp_packet() {
    let mut receiver = RtpReceiver::new("127.0.0.1:0".parse().unwrap())
        .await
        .expect("Failed to create receiver");
    let receiver_addr = receiver.local_addr().expect("Failed to get receiver address");

    // Send invalid packet (RTP header must be at least 12 bytes)
    let invalid_packet = vec![0u8; 5];
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0".parse::<std::net::SocketAddr>().unwrap())
        .await
        .expect("Failed to create test socket");
    socket
        .send_to(&invalid_packet, receiver_addr)
        .await
        .expect("Failed to send invalid packet");

    // Give time for packet to arrive
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Try to receive - should fail with RtpError
    let result = timeout(Duration::from_millis(100), receiver.receive())
        .await
        .expect("receive should not timeout");

    assert!(result.is_err(), "Expected error when receiving invalid RTP packet");

    match result {
        Err(AudioBridgeError::RtpError(msg)) => {
            assert!(
                msg.contains("too short") || msg.contains("invalid"),
                "Expected packet size error, got: {}",
                msg
            );
        }
        Err(e) => panic!("Expected RtpError, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}
