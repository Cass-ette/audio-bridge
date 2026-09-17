#[cfg(target_os = "macos")]
mod playback;

#[cfg(target_os = "macos")]
pub use playback::CoreAudioPlayback;
