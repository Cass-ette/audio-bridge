#!/bin/bash
# Generate test WAV files for audio-io testing
# Requires sox: brew install sox

set -e

OUTPUT_DIR="$(dirname "$0")"

echo "Generating test audio files..."

# 1 second sine wave at 440Hz, 48kHz sample rate, stereo
sox -n -r 48000 -c 2 -b 16 "$OUTPUT_DIR/sine_440hz_1s.wav" synth 1 sine 440

# 5 second silence, 48kHz sample rate, stereo
sox -n -r 48000 -c 2 -b 16 "$OUTPUT_DIR/silence_5s.wav" trim 0 5

# 2 second pink noise, 48kHz sample rate, stereo
sox -n -r 48000 -c 2 -b 16 "$OUTPUT_DIR/pink_noise_2s.wav" synth 2 pinknoise

# Sweep from 100Hz to 2000Hz over 3 seconds
sox -n -r 48000 -c 2 -b 16 "$OUTPUT_DIR/sweep_100_2000hz_3s.wav" synth 3 sine 100-2000

echo "Generated test files:"
ls -lh "$OUTPUT_DIR"/*.wav 2>/dev/null || echo "No WAV files generated"
