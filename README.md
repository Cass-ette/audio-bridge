# Audio Bridge

> ⚠️ **WORK IN PROGRESS** - This project is in early development and has known critical issues. Not ready for production use.

Low-latency audio streaming from Windows to Mac using RTP/Opus.

## ⚠️ Known Issues (Active Development)

This is an early prototype with significant problems:

- **High latency** (~30 seconds) - buffer management needs complete rework
- **Audio quality issues** - Opus encoding/decoding configuration needs tuning
- **Volume problems** - Currently using 10x gain workaround, needs proper level handling
- **Intermittent dropouts** - Receiver pacing and jitter buffer need optimization
- **No error recovery** - Network issues or device changes cause crashes

**Do not use this in production.** The code is being actively developed and will undergo major architectural changes.

## What Works

- ✅ Windows WASAPI loopback capture (captures system audio)
- ✅ macOS CoreAudio playback
- ✅ RTP/Opus streaming over UDP
- ✅ Basic jitter buffer for packet reordering

## Prerequisites

**System Dependencies:**
- `libopus` (Opus audio codec library)

**Install on macOS:**
```bash
brew install opus
```

**Install on Windows:**
- Download pre-built binaries from https://opus-codec.org/downloads/
- Or use vcpkg: `vcpkg install opus`

## Architecture

- `crates/core`: Core audio processing (RTP, Opus, jitter buffer)
- `crates/audio-io`: Cross-platform audio I/O (CoreAudio, WASAPI) ✅
- `crates/sender`: Windows audio capture and sender (TODO)
- `crates/receiver`: Mac audio receiver and playback (TODO)
- `tauri-app`: Cross-platform GUI (TODO)

## Development Status

### Current Phase: Integration & Testing (2026-09-18)

**Recent work:**
- Integrated Windows WASAPI loopback with RTP sender
- Integrated macOS CoreAudio with RTP receiver  
- End-to-end streaming works but has critical performance issues (see above)

### Phase 1c: Platform Audio I/O ✅ Complete (2026-09-18)

Implemented native audio capture and playback:
- **macOS**: CoreAudio (AudioUnit) backend
- **Windows**: WASAPI (Windows Audio Session API) backend
- **API**: Unified traits for cross-platform audio I/O
- **Examples**: Sine wave, mic loopback, device enumeration, system audio streaming
- **Documentation**: See `crates/audio-io/README.md`

For detailed completion notes, see `docs/superpowers/specs/phase1c-completion-notes.md`.

## Quick Start (Testing Only)

**On Windows (sender):**
```powershell
cd C:\path\to\audio-bridge
cargo run --release --example loopback_sender -- --target <mac-ip>:5004
```

**On macOS (receiver):**
```bash
cargo run --release --example receiver -- --bind 0.0.0.0:5004
```

Play audio on Windows and it should stream to macOS (with the issues mentioned above).

## Build

```bash
cargo build
cargo test
```

## License

MIT
