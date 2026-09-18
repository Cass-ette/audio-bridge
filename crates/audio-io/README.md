# audio-io

Cross-platform audio I/O abstraction for low-latency audio capture and playback.

## Features

- **Unified API**: Single trait-based interface across platforms
- **Platform-native backends**:
  - **macOS**: CoreAudio (AudioUnit)
  - **Windows**: WASAPI (Windows Audio Session API)
- **Low-latency**: Optimized for real-time audio streaming
- **Non-blocking**: Callback-based architecture
- **Error handling**: Comprehensive error types with platform-specific details

## Platform Support

| Platform | Backend | Status | Notes |
|----------|---------|--------|-------|
| macOS    | CoreAudio | ✅ Implemented | Uses AudioUnit API |
| Windows  | WASAPI   | ✅ Implemented | Exclusive mode for lowest latency |
| Linux    | -        | ❌ Not planned | Future: ALSA/PulseAudio/JACK |

## Usage

### Basic Playback

```rust
use audio_io::{AudioOutput, AudioFormat, AudioDeviceConfig};
use std::sync::{Arc, Mutex};

// Define audio format
let format = AudioFormat {
    sample_rate: 48000,
    channels: 2,
    bits_per_sample: 16,
};

// Create output device
let config = AudioDeviceConfig::default();
let output = AudioOutput::new(&format, config)?;

// Audio callback
let buffer_data = Arc::new(Mutex::new(vec![0i16; 4096]));
let buffer_clone = buffer_data.clone();

output.start(move |out_buffer| {
    let data = buffer_clone.lock().unwrap();
    let len = out_buffer.len().min(data.len());
    out_buffer[..len].copy_from_slice(&data[..len]);
})?;

// Play audio...

output.stop()?;
```

### Basic Capture

```rust
use audio_io::{AudioInput, AudioFormat, AudioDeviceConfig};
use std::sync::{Arc, Mutex};

let format = AudioFormat {
    sample_rate: 48000,
    channels: 2,
    bits_per_sample: 16,
};

let config = AudioDeviceConfig::default();
let input = AudioInput::new(&format, config)?;

let captured = Arc::new(Mutex::new(Vec::new()));
let captured_clone = captured.clone();

input.start(move |in_buffer| {
    captured_clone.lock().unwrap().extend_from_slice(in_buffer);
})?;

// Capture audio...

input.stop()?;
```

### Device Enumeration

```rust
use audio_io::DeviceManager;

let manager = DeviceManager::new()?;

// List output devices
println!("Output devices:");
for device in manager.list_output_devices()? {
    println!("  - {} (default: {})", device.name, device.is_default);
}

// List input devices
println!("Input devices:");
for device in manager.list_input_devices()? {
    println!("  - {} (default: {})", device.name, device.is_default);
}

// Get default devices
let default_output = manager.get_default_output_device()?;
let default_input = manager.get_default_input_device()?;
```

## Audio Format Specifications

### Supported Formats

- **Sample rates**: 8000, 16000, 22050, 44100, 48000, 96000 Hz
- **Channels**: 1 (mono), 2 (stereo)
- **Bit depth**: 16-bit signed integer (PCM)

### Buffer Format

Audio data is represented as `&[i16]` (signed 16-bit PCM):
- **Interleaved**: For stereo, samples are L-R-L-R...
- **Sample range**: -32768 to 32767
- **Endianness**: Platform native

## Configuration

```rust
use audio_io::AudioDeviceConfig;

let config = AudioDeviceConfig {
    buffer_size: 512,        // Buffer size in frames (samples per channel)
    device_id: None,         // None = default device, Some(id) = specific device
};
```

### Buffer Size Guidelines

| Latency Target | Recommended Buffer Size (frames) | Total Latency (approx @ 48kHz) |
|----------------|----------------------------------|--------------------------------|
| Ultra-low      | 64-128                           | 1.3-2.7 ms                     |
| Low            | 256-512                          | 5.3-10.7 ms                    |
| Normal         | 1024                             | 21.3 ms                        |
| High           | 2048+                            | 42.7+ ms                       |

**Note**: Smaller buffers = lower latency but higher CPU usage and potential for underruns/overruns.

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run with audio output (requires audio hardware)
cargo test -- --nocapture

# Run specific test
cargo test test_output_device
```

### Generating Test Audio

```bash
cd test_data
./generate_test_wav.sh  # Requires sox: brew install sox
```

This generates:
- `sine_440hz_1s.wav`: 1s 440Hz sine wave
- `silence_5s.wav`: 5s silence
- `pink_noise_2s.wav`: 2s pink noise
- `sweep_100_2000hz_3s.wav`: 3s frequency sweep

### Running Examples

```bash
# Sine wave generator (playback test)
cargo run --example sine_wave

# Microphone loopback (capture + playback test)
cargo run --example mic_loopback

# Device enumeration
cargo run --example list_devices
```

## Platform-Specific Notes

### macOS

- **Permissions**: Microphone access requires user permission (macOS will prompt)
- **AudioUnit**: Uses kAudioUnitSubType_HALOutput and kAudioUnitSubType_HALInput
- **Sample rate**: Device native sample rate is preferred; resampling may occur
- **Thread priority**: Audio callbacks run on high-priority real-time thread

### Windows

- **WASAPI Mode**: Uses exclusive mode for lowest latency
- **COM initialization**: Automatically handled per-thread
- **Permissions**: No special permissions required
- **Sample rate**: Uses device's current mix format
- **Thread priority**: Audio callbacks run on WASAPI's MMCSS thread

## Error Handling

```rust
use audio_io::{AudioOutput, AudioError};

match AudioOutput::new(&format, config) {
    Ok(output) => { /* use output */ },
    Err(AudioError::UnsupportedFormat) => {
        eprintln!("Audio format not supported by device");
    },
    Err(AudioError::DeviceNotFound) => {
        eprintln!("Audio device not found");
    },
    Err(AudioError::Platform(msg)) => {
        eprintln!("Platform error: {}", msg);
    },
    Err(e) => {
        eprintln!("Audio error: {:?}", e);
    },
}
```

## Known Limitations

- **No resampling**: Input sample rate must match device native rate
- **Fixed format**: Only 16-bit PCM supported (no 24-bit, 32-bit float)
- **No device hotplug**: Device changes require restart
- **Linux**: Not yet implemented

## Integration with audio-bridge

This crate provides the I/O layer for the audio-bridge project:

1. **Sender** (Windows): Uses `AudioInput` to capture system audio
2. **Receiver** (Mac): Uses `AudioOutput` to play back received audio
3. **Format**: 48kHz, stereo, 16-bit matches Opus codec input

## License

MIT
