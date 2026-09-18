#[cfg(target_os = "macos")]
mod playback;

#[cfg(target_os = "macos")]
mod capture;

#[cfg(target_os = "macos")]
pub use playback::CoreAudioPlayback;

#[cfg(target_os = "macos")]
pub use capture::CoreAudioCapture;
