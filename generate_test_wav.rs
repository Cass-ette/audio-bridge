use hound::{WavWriter, WavSpec};

fn main() {
    let spec = WavSpec {
        channels: 2,
        sample_rate: 48000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    
    let path = "crates/audio-io/test_data/test_48k_stereo.wav";
    let mut writer = WavWriter::create(path, spec).unwrap();
    
    // Generate 10 seconds of 440Hz sine wave
    let sample_rate = 48000.0;
    let frequency = 440.0;
    
    for i in 0..(48000 * 10 * 2) { // 10 seconds stereo
        let t = (i / 2) as f32 / sample_rate;
        let value = (2.0 * std::f32::consts::PI * frequency * t).sin();
        let sample = (value * 8000.0) as i16;
        writer.write_sample(sample).unwrap();
    }
    
    writer.finalize().unwrap();
    println!("Generated: {}", path);
}
