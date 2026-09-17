use crate::Result;

/// Audio capture interface (platform-specific implementations)
pub trait AudioCapture: Send {
    /// Start audio capture
    fn start(&mut self) -> Result<()>;

    /// Read audio samples
    ///
    /// # Arguments
    /// * `buffer` - Target buffer (1920 samples for 20ms stereo at 48kHz)
    ///
    /// # Returns
    /// Number of samples actually read
    fn read(&mut self, buffer: &mut [i16]) -> Result<usize>;

    /// Stop audio capture
    fn stop(&mut self) -> Result<()>;
}

/// Audio playback interface (platform-specific implementations)
pub trait AudioPlayback: Send {
    /// Start audio playback
    fn start(&mut self) -> Result<()>;

    /// Write audio samples for playback
    ///
    /// # Arguments
    /// * `buffer` - PCM data (1920 samples for 20ms stereo at 48kHz)
    fn write(&mut self, buffer: &[i16]) -> Result<()>;

    /// Stop audio playback
    fn stop(&mut self) -> Result<()>;
}
