//! Loopback audio sender (Virtual Sound Card)
//!
//! Captures system audio output in real-time and sends it over RTP.
//! On Windows, uses WASAPI loopback to capture whatever is playing.
//!
//! Usage:
//!   cargo run --example loopback_sender -- --target <ip:port>
//!
//! Example:
//!   cargo run --example loopback_sender -- --target 192.168.8.45:5004

use audio_bridge_io::{AudioSender, SenderConfig};
use std::net::SocketAddr;
use std::sync::Arc;

#[cfg(target_os = "windows")]
fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 || args[1] != "--target" {
        eprintln!("Usage: {} --target <ip:port>", args[0]);
        eprintln!("Example: {} --target 192.168.8.45:5004", args[0]);
        std::process::exit(1);
    }

    let target_addr: SocketAddr = args[2].parse().unwrap_or_else(|_| {
        eprintln!("Invalid target address: {}", args[2]);
        std::process::exit(1);
    });

    // Create tokio runtime for async initialization
    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Create sender with WASAPI loopback capture
    let config = SenderConfig::default();
    let mut sender = runtime
        .block_on(async { AudioSender::new(target_addr, config).await })
        .unwrap_or_else(|e| {
            eprintln!("Failed to create audio sender: {:?}", e);
            eprintln!("\nNote: Make sure audio is currently playing on your system.");
            std::process::exit(1);
        });

    println!("🎵 Virtual Sound Card - WASAPI Loopback Sender");
    println!("================================================");
    println!("  Source: System Audio Output (Loopback)");
    println!("  Target: {}", target_addr);
    println!("  Config: 48kHz stereo, Opus encoding");
    println!();
    println!("💡 This captures ALL audio playing on your system.");
    println!("   Play some music/video to test the transmission.");
    println!();
    println!("Starting transmission... Press Ctrl+C to stop.");
    println!();

    // Get running handle for Ctrl+C handler
    let running_handle = sender.running_handle();

    // Setup Ctrl+C handler
    let running_clone = Arc::clone(&running_handle);
    ctrlc::set_handler(move || {
        println!("\n\nReceived Ctrl+C, stopping...");
        running_clone.store(false, std::sync::atomic::Ordering::SeqCst);
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
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("❌ Loopback capture is only supported on Windows.");
    eprintln!("   On macOS/Linux, use the file-based sender instead:");
    eprintln!("   cargo run --example sender -- --source <file.wav> --target <ip:port>");
    std::process::exit(1);
}
