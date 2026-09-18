//! Audio receiver example
//!
//! Receives audio over RTP and plays it through the default audio device.
//!
//! Usage:
//!   cargo run --example receiver -- --bind <ip:port>
//!
//! Example:
//!   cargo run --example receiver -- --bind 127.0.0.1:8000

use audio_bridge_io::{AudioReceiver, ReceiverConfig};
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 || args[1] != "--bind" {
        eprintln!("Usage: {} --bind <ip:port>", args[0]);
        eprintln!("Example: {} --bind 127.0.0.1:8000", args[0]);
        std::process::exit(1);
    }

    let bind_addr: SocketAddr = args[2].parse().unwrap_or_else(|_| {
        eprintln!("Invalid bind address: {}", args[2]);
        std::process::exit(1);
    });

    // Create receiver with platform-default playback
    let config = ReceiverConfig::default();
    let mut receiver = match AudioReceiver::new(bind_addr, config).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to create audio receiver: {:?}", e);
            eprintln!("\nNote: Platform-default playback may not be available on all systems.");
            eprintln!("On non-macOS platforms, use AudioReceiver::with_playback() instead.");
            std::process::exit(1);
        }
    };

    println!("Audio Receiver");
    println!("  Bind address: {}", bind_addr);
    println!("  Config: 48kHz stereo, Opus decoding");
    println!();
    println!("Waiting for audio... Press Ctrl+C to stop.");

    // Get running handle for Ctrl+C handler
    let running_handle = receiver.running_handle();

    // Setup Ctrl+C handler
    let running_clone = Arc::clone(&running_handle);
    ctrlc::set_handler(move || {
        println!("\nReceived Ctrl+C, stopping...");
        running_clone.store(false, std::sync::atomic::Ordering::SeqCst);
    })
    .expect("Error setting Ctrl+C handler");

    // Start receiver (async)
    match receiver.start().await {
        Ok(()) => {
            println!("\nReceiver stopped.");
        }
        Err(e) => {
            eprintln!("\nReceiver failed: {:?}", e);
            std::process::exit(1);
        }
    }

    // Print statistics
    let stats = receiver.stats();
    println!("\nStatistics:");
    println!("  Packets received: {}", stats.packets_received);
    println!("  Packets decoded: {}", stats.packets_decoded);
    println!("  Packets lost: {}", stats.packets_lost);
    println!("  Bytes received: {}", stats.bytes_received);
    println!("  Decoding errors: {}", stats.decoding_errors);
}
