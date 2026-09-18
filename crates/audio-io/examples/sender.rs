//! Audio sender example
//!
//! Reads audio from a WAV file and sends it over RTP to a target address.
//!
//! Usage:
//!   cargo run --example sender -- --source <wav-file> --target <ip:port>
//!
//! Example:
//!   cargo run --example sender -- --source test.wav --target 127.0.0.1:8000

use audio_bridge_io::mock::FileAudioSource;
use audio_bridge_io::{AudioSender, SenderConfig};
use std::net::SocketAddr;
use std::sync::Arc;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 || args[1] != "--source" || args[3] != "--target" {
        eprintln!("Usage: {} --source <wav-file> --target <ip:port>", args[0]);
        eprintln!("Example: {} --source test.wav --target 127.0.0.1:8000", args[0]);
        std::process::exit(1);
    }

    let source_path = &args[2];
    let target_addr: SocketAddr = args[4].parse().unwrap_or_else(|_| {
        eprintln!("Invalid target address: {}", args[4]);
        std::process::exit(1);
    });

    // Create file audio source
    let capture = match FileAudioSource::open(source_path) {
        Ok(source) => Box::new(source),
        Err(e) => {
            eprintln!("Failed to open audio file '{}': {:?}", source_path, e);
            std::process::exit(1);
        }
    };

    // Create tokio runtime for async initialization
    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Create sender
    let config = SenderConfig::default();
    let mut sender = runtime.block_on(async {
        AudioSender::with_capture(capture, target_addr, config).await
    }).unwrap_or_else(|e| {
        eprintln!("Failed to create audio sender: {:?}", e);
        std::process::exit(1);
    });

    println!("Audio Sender");
    println!("  Source: {}", source_path);
    println!("  Target: {}", target_addr);
    println!("  Config: 48kHz stereo, Opus encoding");
    println!();
    println!("Starting transmission... Press Ctrl+C to stop.");

    // Get running handle for Ctrl+C handler
    let running_handle = sender.running_handle();

    // Setup Ctrl+C handler
    let running_clone = Arc::clone(&running_handle);
    ctrlc::set_handler(move || {
        println!("\nReceived Ctrl+C, stopping...");
        running_clone.store(false, std::sync::atomic::Ordering::SeqCst);
    })
    .expect("Error setting Ctrl+C handler");

    // Start sender (blocking)
    match sender.start() {
        Ok(()) => {
            println!("\nTransmission completed.");
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
