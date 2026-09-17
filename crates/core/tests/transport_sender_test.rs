use audio_bridge_core::{transport::RtpSender, Result};
use std::net::SocketAddr;

#[tokio::test]
async fn test_sender_creation() -> Result<()> {
    let target: SocketAddr = "127.0.0.1:5004".parse().unwrap();
    let sender = RtpSender::new(target).await?;

    // Verify sender is bound to some ephemeral port
    assert!(sender.local_addr()?.port() > 0);

    Ok(())
}
