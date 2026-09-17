use crate::traits::AudioPlayback;
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

// SAFETY: Windows COM types are safe to send across threads when properly initialized
// with COINIT_MULTITHREADED. Each WasapiCapture instance owns its COM objects.
unsafe impl Send for CoreAudioPlayback {}
