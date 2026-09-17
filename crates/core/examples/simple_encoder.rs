//! Simple example: encode silence to Opus and print stats

use audio_bridge_core::codec::{AudioFormat, OpusEncoder};

fn main() -> audio_bridge_core::Result<()> {
    println!("Audio Bridge Core - Simple Encoder Example\n");

    let format = AudioFormat {
        sample_rate: 48000,
        channels: 2,
        bitrate: 128000,
    };

    println!("Creating Opus encoder:");
    println!("  Sample rate: {} Hz", format.sample_rate);
    println!("  Channels: {}", format.channels);
    println!("  Bitrate: {} kbps\n", format.bitrate / 1000);

    let mut encoder = OpusEncoder::new(format)?;

    // Generate 1 second of silence (50 frames * 20ms)
    let frame_size = 960; // 20ms at 48kHz
    let pcm = vec![0i16; frame_size * format.channels as usize];

    println!("Encoding 50 frames (1 second) of silence...");

    let mut total_input_bytes = 0;
    let mut total_output_bytes = 0;

    for i in 0..50 {
        let encoded = encoder.encode(&pcm)?;

        total_input_bytes += pcm.len() * 2; // i16 = 2 bytes
        total_output_bytes += encoded.len();

        if i % 10 == 0 {
            println!("  Frame {}: {} bytes PCM → {} bytes Opus",
                     i, pcm.len() * 2, encoded.len());
        }
    }

    println!("\nResults:");
    println!("  Total input:  {} KB", total_input_bytes / 1024);
    println!("  Total output: {} KB", total_output_bytes / 1024);
    println!("  Compression ratio: {:.1}x",
             total_input_bytes as f32 / total_output_bytes as f32);

    Ok(())
}
