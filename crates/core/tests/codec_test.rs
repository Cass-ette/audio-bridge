use audio_bridge_core::codec::{AudioFormat, OpusDecoder, OpusEncoder};

#[test]
fn test_opus_encoder_create() {
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 2,
        bitrate: 128000,
    };

    let encoder = OpusEncoder::new(format).unwrap();
    assert_eq!(encoder.format().sample_rate, 48000);
    assert_eq!(encoder.format().channels, 2);
}

#[test]
fn test_opus_encode_silence() {
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 2,
        bitrate: 128000,
    };

    let mut encoder = OpusEncoder::new(format).unwrap();

    // 20ms of silence: 48000 samples/sec * 0.02 sec * 2 channels = 1920 samples
    let pcm = vec![0i16; 1920];
    let encoded = encoder.encode(&pcm).unwrap();

    // Opus compressed silence should be much smaller than PCM
    assert!(encoded.len() < pcm.len() * 2);
    assert!(encoded.len() > 0);
}

#[test]
fn test_opus_decoder_create() {
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 2,
        bitrate: 128000,
    };

    let decoder = OpusDecoder::new(format).unwrap();
    assert_eq!(decoder.format().sample_rate, 48000);
}

#[test]
fn test_opus_encode_decode_roundtrip() {
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 2,
        bitrate: 128000,
    };

    let mut encoder = OpusEncoder::new(format).unwrap();
    let mut decoder = OpusDecoder::new(format).unwrap();

    // Generate test signal: 440Hz sine wave (A note)
    let frame_size = 960; // 20ms at 48kHz
    let mut pcm = Vec::with_capacity(frame_size * 2);
    for i in 0..frame_size {
        let t = i as f32 / 48000.0;
        let sample = (2.0 * std::f32::consts::PI * 440.0 * t).sin();
        let sample_i16 = (sample * 32767.0) as i16;
        pcm.push(sample_i16); // Left
        pcm.push(sample_i16); // Right
    }

    // Encode
    let encoded = encoder.encode(&pcm).unwrap();
    assert!(encoded.len() > 0);
    assert!(encoded.len() < pcm.len() * 2); // Compressed

    // Decode
    let decoded = decoder.decode(&encoded, false).unwrap();
    assert_eq!(decoded.len(), pcm.len());

    // Opus has algorithmic delay (lookahead ~312 samples per channel = 624 total for stereo)
    // The decoded signal is delayed by this amount, so we compare:
    // decoded[lookahead..] with original[0..len-lookahead]
    let lookahead_offset = 624; // 312 samples per channel * 2 channels

    let mut max_error = 0i32;
    let mut total_error = 0i64;
    let mut count = 0i64;

    for (&orig, &dec) in pcm
        .iter()
        .take(pcm.len() - lookahead_offset)
        .zip(decoded.iter().skip(lookahead_offset))
    {
        let diff = (orig as i32 - dec as i32).abs();
        max_error = max_error.max(diff);
        total_error += diff as i64;
        count += 1;
    }
    let avg_error = if count > 0 { total_error / count } else { 0 };

    println!("Max error: {}, Avg error: {}", max_error, avg_error);

    // Opus is lossy but should be reasonably accurate after alignment
    assert!(max_error < 5000, "Max error {} too high", max_error);
    assert!(avg_error < 1000, "Avg error {} too high", avg_error);
}
