use crate::traits::AudioPlayback;
use crate::{AudioIoError, Result};
use coreaudio_sys::*;
use std::collections::VecDeque;
use std::os::raw::c_void;
use std::sync::{Arc, Mutex};

pub struct CoreAudioPlayback {
    audio_queue: AudioQueueRef,
    buffers: Vec<AudioQueueBufferRef>,
    format: AudioStreamBasicDescription,
    buffer_size: usize,
    playback_queue: Arc<Mutex<VecDeque<Vec<i16>>>>,
    user_data: *mut c_void,
    started: bool,
}

// SAFETY: CoreAudio AudioQueue is thread-safe and can be used across threads
unsafe impl Send for CoreAudioPlayback {}

// Callback function that fills audio buffers from playback_queue
unsafe extern "C" fn playback_callback(
    user_data: *mut c_void,
    queue: AudioQueueRef,
    buffer: AudioQueueBufferRef,
) {
    if user_data.is_null() || buffer.is_null() {
        return;
    }

    // Retrieve the Arc from user_data (it was stored via Arc::into_raw)
    let playback_queue = &*(user_data as *const Mutex<VecDeque<Vec<i16>>>);

    let mut queue_guard = match playback_queue.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };

    let buffer_ref = &mut *buffer;
    let buffer_size = buffer_ref.mAudioDataBytesCapacity as usize;
    let sample_count = buffer_size / 2; // 16-bit samples
    let dest = std::slice::from_raw_parts_mut(buffer_ref.mAudioData as *mut i16, sample_count);

    let mut written = 0;
    while written < sample_count {
        if let Some(chunk) = queue_guard.front() {
            let to_copy = (sample_count - written).min(chunk.len());
            dest[written..written + to_copy].copy_from_slice(&chunk[..to_copy]);
            written += to_copy;

            if to_copy == chunk.len() {
                queue_guard.pop_front();
            } else {
                // Partial consumption - keep remaining data
                let remaining = chunk[to_copy..].to_vec();
                queue_guard.pop_front();
                queue_guard.push_front(remaining);
                break;
            }
        } else {
            // No data available - fill with silence
            dest[written..].fill(0);
            break;
        }
    }

    buffer_ref.mAudioDataByteSize = (written * 2) as u32;

    // Re-enqueue the buffer
    AudioQueueEnqueueBuffer(queue, buffer, 0, std::ptr::null());
}

impl CoreAudioPlayback {
    /// Create a new CoreAudio playback instance
    ///
    /// # Returns
    /// A new CoreAudioPlayback instance configured for 48kHz stereo 16-bit PCM
    pub fn new() -> Result<Self> {
        unsafe {
            // Create AudioStreamBasicDescription for 48kHz stereo 16-bit PCM
            let format = AudioStreamBasicDescription {
                mSampleRate: 48000.0,
                mFormatID: kAudioFormatLinearPCM,
                mFormatFlags: kLinearPCMFormatFlagIsSignedInteger | kLinearPCMFormatFlagIsPacked,
                mBytesPerPacket: 4,    // 2 channels * 2 bytes
                mFramesPerPacket: 1,
                mBytesPerFrame: 4,     // 2 channels * 2 bytes
                mChannelsPerFrame: 2,  // Stereo
                mBitsPerChannel: 16,
                mReserved: 0,
            };

            let playback_queue = Arc::new(Mutex::new(VecDeque::new()));
            let user_data = Arc::into_raw(playback_queue.clone()) as *mut c_void;

            // Create AudioQueue
            let mut audio_queue: AudioQueueRef = std::ptr::null_mut();
            let status = AudioQueueNewOutput(
                &format,
                Some(playback_callback),
                user_data,
                std::ptr::null_mut(), // CFRunLoop - null means use internal thread
                std::ptr::null_mut(), // CFRunLoopMode
                0,
                &mut audio_queue,
            );

            if status != 0 {
                // Reclaim the Arc to prevent leak
                let _ = Arc::from_raw(user_data as *const Mutex<VecDeque<Vec<i16>>>);
                return Err(AudioIoError::DeviceOpenFailed(format!(
                    "AudioQueueNewOutput failed with status: {}",
                    status
                )));
            }

            // Allocate and enqueue 3 buffers
            let buffer_size = 1920 * 2; // 20ms stereo at 48kHz in bytes (1920 samples * 2 bytes)
            let mut buffers = Vec::new();

            for _ in 0..3 {
                let mut buffer: AudioQueueBufferRef = std::ptr::null_mut();
                let status = AudioQueueAllocateBuffer(audio_queue, buffer_size as u32, &mut buffer);

                if status != 0 {
                    // Clean up already allocated buffers
                    for buf in buffers {
                        AudioQueueFreeBuffer(audio_queue, buf);
                    }
                    AudioQueueDispose(audio_queue, 1);
                    let _ = Arc::from_raw(user_data as *const Mutex<VecDeque<Vec<i16>>>);
                    return Err(AudioIoError::DeviceOpenFailed(format!(
                        "AudioQueueAllocateBuffer failed with status: {}",
                        status
                    )));
                }

                // Initialize buffer with silence and enqueue
                let buffer_ref = &mut *buffer;
                buffer_ref.mAudioDataByteSize = buffer_size as u32;
                std::ptr::write_bytes(buffer_ref.mAudioData, 0, buffer_size);

                buffers.push(buffer);
            }

            // Don't enqueue buffers yet - they'll be enqueued when we start
            Ok(CoreAudioPlayback {
                audio_queue,
                buffers,
                format,
                buffer_size,
                playback_queue,
                user_data,
                started: false,
            })
        }
    }
}

impl AudioPlayback for CoreAudioPlayback {
    fn start(&mut self) -> Result<()> {
        if self.started {
            return Ok(());
        }

        unsafe {
            // Enqueue all buffers before starting
            for &buffer in &self.buffers {
                let buffer_ref = &mut *buffer;
                buffer_ref.mAudioDataByteSize = self.buffer_size as u32;
                std::ptr::write_bytes(buffer_ref.mAudioData, 0, self.buffer_size);

                let status = AudioQueueEnqueueBuffer(self.audio_queue, buffer, 0, std::ptr::null());
                if status != 0 {
                    return Err(AudioIoError::Platform(format!(
                        "AudioQueueEnqueueBuffer failed with status: {}",
                        status
                    )));
                }
            }

            let status = AudioQueueStart(self.audio_queue, std::ptr::null());
            if status != 0 {
                return Err(AudioIoError::Platform(format!(
                    "AudioQueueStart failed with status: {}",
                    status
                )));
            }
        }

        self.started = true;
        Ok(())
    }

    fn write(&mut self, buffer: &[i16]) -> Result<()> {
        let mut queue = self
            .playback_queue
            .lock()
            .map_err(|e| AudioIoError::Platform(format!("Lock error: {}", e)))?;

        // Limit queue size to prevent unbounded memory growth
        if queue.len() >= 10 {
            return Err(AudioIoError::BufferOverrun);
        }

        queue.push_back(buffer.to_vec());
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        if !self.started {
            return Ok(());
        }

        unsafe {
            let status = AudioQueueStop(self.audio_queue, 1); // 1 = stop immediately
            if status != 0 {
                return Err(AudioIoError::Platform(format!(
                    "AudioQueueStop failed with status: {}",
                    status
                )));
            }
        }

        self.started = false;
        Ok(())
    }
}

impl Drop for CoreAudioPlayback {
    fn drop(&mut self) {
        unsafe {
            // Stop the queue if still running
            if self.started {
                AudioQueueStop(self.audio_queue, 1);
            }

            // Free buffers
            for buffer in &self.buffers {
                AudioQueueFreeBuffer(self.audio_queue, *buffer);
            }

            // Dispose queue
            AudioQueueDispose(self.audio_queue, 1);

            // Reclaim Arc from user_data to properly drop it
            if !self.user_data.is_null() {
                let _ = Arc::from_raw(self.user_data as *const Mutex<VecDeque<Vec<i16>>>);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn test_coreaudio_create() {
        // Simple test to verify we can create an instance
        let playback = CoreAudioPlayback::new();
        assert!(playback.is_ok(), "Failed to create CoreAudioPlayback: {:?}", playback.err());
        println!("CoreAudioPlayback created successfully");
    }

    #[test]
    fn test_coreaudio_sine_wave() {
        // Generate 440Hz sine wave for 1 second
        let sample_rate = 48000;
        let duration_secs = 1;
        let frequency = 440.0;
        let amplitude = 0.3; // 30% volume to avoid clipping

        let total_samples = sample_rate * duration_secs;
        let mut samples = Vec::with_capacity(total_samples);

        for i in 0..total_samples {
            let t = i as f32 / sample_rate as f32;
            let sample = (2.0 * PI * frequency * t).sin() * amplitude * i16::MAX as f32;
            samples.push(sample as i16);
        }

        // Create playback instance
        let mut playback = CoreAudioPlayback::new().expect("Failed to create CoreAudioPlayback");

        // Start playback first
        playback.start().expect("Failed to start playback");

        // Write samples in 20ms chunks (1920 samples for stereo)
        let chunk_size = 1920; // 20ms stereo at 48kHz
        for chunk in samples.chunks(chunk_size / 2) {
            // Convert mono to stereo by duplicating samples
            let mut stereo_chunk = Vec::with_capacity(chunk.len() * 2);
            for &sample in chunk {
                stereo_chunk.push(sample);
                stereo_chunk.push(sample);
            }
            playback
                .write(&stereo_chunk)
                .expect("Failed to write audio");

            // Small delay to avoid filling the queue too fast
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Sleep to let remaining audio play
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Stop playback
        playback.stop().expect("Failed to stop playback");

        println!("Sine wave test completed - you should have heard a 440Hz tone");
    }
}
