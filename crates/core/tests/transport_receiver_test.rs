use audio_bridge_core::{transport::RtpReceiver, Result};
use std::net::SocketAddr;

#[tokio::test]
async fn test_receiver_creation() -> Result<()> {
    let bind_addr: SocketAddr = "127.0.0.1:5004".parse().unwrap();
    let receiver = RtpReceiver::new(bind_addr).await?;

    assert_eq!(receiver.local_addr()?.port(), 5004);

    Ok(())
}
