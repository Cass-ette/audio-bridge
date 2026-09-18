#[cfg(target_os = "windows")]
mod capture;

#[cfg(target_os = "windows")]
pub use capture::WasapiCapture;
