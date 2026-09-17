use audio_bridge_core::codec::{OpusEncoder, AudioFormat};

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
