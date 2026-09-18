//! macOS microphone sender example
//!
//! Captures audio from the default microphone and sends it via RTP.

use audio_bridge_io::{
    macos::CoreAudioCapture, AudioSender, Bitrate, Result, SenderConfig,
};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 || args[1] != "--target" {
        eprintln!("Usage: {} --target <ip:port>", args[0]);
        eprintln!("Example: {} --target 192.168.1.100:5004", args[0]);
        std::process::exit(1);
    }

    let target: SocketAddr = args[2]
        .parse()
        .expect("Invalid target address format");

    println!("🎤 macOS Microphone Sender");
    println!("==========================");
    println!("  Source: Default Microphone");
    println!("  Target: {}", target);
    println!("  Config: 48kHz stereo, Opus encoding");
    println!();
    println!("💡 Speak into your microphone to test the transmission.");
    println!();
    println!("Starting transmission... Press Ctrl+C to stop.");
    println!();

    // Create capture device
    let capture = CoreAudioCapture::new(None)?;

    // Create sender config
    let config = SenderConfig {
        bitrate: Bitrate::Kbps128,
        capture_device: None,
        ssrc: rand::random(),
    };

    // Create sender
    let mut sender = AudioSender::with_capture(Box::new(capture), target, config).await?;

    // Setup Ctrl+C handler
    let running = sender.running_handle();
    ctrlc::set_handler(move || {
        println!("\nCtrl+C received, stopping...");
        running.store(false, std::sync::atomic::Ordering::SeqCst);
    })
    .expect("Error setting Ctrl+C handler");

    // Start sender (blocking)
    match sender.start() {
        Ok(()) => {
            println!("\nTransmission stopped.");
        }
        Err(e) => {
            eprintln!("\nTransmission failed: {:?}", e);
            std::process::exit(1);
        }
    }

    // Print statistics
    let stats = sender.stats();
    println!("\nStatistics:");
    println!("  Frames captured: {}", stats.frames_captured);
    println!("  Packets sent: {}", stats.packets_sent);
    println!("  Bytes sent: {}", stats.bytes_sent);
    println!("  Encoding errors: {}", stats.encoding_errors);

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("❌ Microphone capture is only supported on macOS in this example.");
    eprintln!("   On Windows, use the loopback_sender instead:");
    eprintln!("   cargo run --example loopback_sender -- --target <ip:port>");
    std::process::exit(1);
}
