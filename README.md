# Audio Bridge

Low-latency audio streaming from Windows to Mac.

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

### Phase 1c: Platform Audio I/O ✅ Complete (2026-09-18)

Implemented native audio capture and playback:
- **macOS**: CoreAudio (AudioUnit) backend
- **Windows**: WASAPI (Windows Audio Session API) backend
- **API**: Unified traits for cross-platform audio I/O
- **Examples**: Sine wave, mic loopback, device enumeration
- **Documentation**: See `crates/audio-io/README.md`

**Next**: Runtime testing on actual hardware, then integration with sender/receiver apps.

For detailed completion notes, see `docs/superpowers/specs/phase1c-completion-notes.md`.

## Build

```bash
cargo build
cargo test
```

## License

MIT
