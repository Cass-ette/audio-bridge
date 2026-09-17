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
- `crates/sender`: Windows audio capture and sender (TODO)
- `crates/receiver`: Mac audio receiver and playback (TODO)
- `tauri-app`: Cross-platform GUI (TODO)

## Build

```bash
cargo build
cargo test
```

## License

MIT
