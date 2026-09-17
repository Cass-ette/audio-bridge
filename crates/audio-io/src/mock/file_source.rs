use crate::{traits::AudioCapture, AudioIoError, Result};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub struct FileAudioSource {
    file: File,
    header: WavHeader,
    samples_read: usize,
    loop_playback: bool,
}

struct WavHeader {
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    data_offset: u64,
    data_size: u32,
}

impl FileAudioSource {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = File::open(path)?;
        let header = Self::parse_wav_header(&mut file)?;

        Ok(Self {
            file,
            header,
            samples_read: 0,
            loop_playback: false,
        })
    }

    pub fn with_loop(mut self, enable: bool) -> Self {
        self.loop_playback = enable;
        self
    }

    fn parse_wav_header(file: &mut File) -> Result<WavHeader> {
        let mut buf = [0u8; 12];
        file.read_exact(&mut buf)?;

        // Check RIFF header
        if &buf[0..4] != b"RIFF" {
            return Err(AudioIoError::WavParse("Not a RIFF file".into()));
        }
        if &buf[8..12] != b"WAVE" {
            return Err(AudioIoError::WavParse("Not a WAVE file".into()));
        }

        // Find fmt chunk
        let mut chunk_header = [0u8; 8];
        loop {
            file.read_exact(&mut chunk_header)?;
            let chunk_id = &chunk_header[0..4];
            let chunk_size = u32::from_le_bytes([
                chunk_header[4],
                chunk_header[5],
                chunk_header[6],
                chunk_header[7],
            ]);

            if chunk_id == b"fmt " {
                let mut fmt_data = vec![0u8; chunk_size as usize];
                file.read_exact(&mut fmt_data)?;

                let audio_format = u16::from_le_bytes([fmt_data[0], fmt_data[1]]);
                if audio_format != 1 {
                    return Err(AudioIoError::WavParse(
                        "Only PCM format is supported".into(),
                    ));
                }

                let channels = u16::from_le_bytes([fmt_data[2], fmt_data[3]]);
                let sample_rate = u32::from_le_bytes([
                    fmt_data[4],
                    fmt_data[5],
                    fmt_data[6],
                    fmt_data[7],
                ]);
                let bits_per_sample = u16::from_le_bytes([fmt_data[14], fmt_data[15]]);

                // Find data chunk
                loop {
                    file.read_exact(&mut chunk_header)?;
                    let data_chunk_id = &chunk_header[0..4];
                    let data_size = u32::from_le_bytes([
                        chunk_header[4],
                        chunk_header[5],
                        chunk_header[6],
                        chunk_header[7],
                    ]);

                    if data_chunk_id == b"data" {
                        let data_offset = file.stream_position()?;
                        return Ok(WavHeader {
                            sample_rate,
                            channels,
                            bits_per_sample,
                            data_offset,
                            data_size,
                        });
                    } else {
                        // Skip unknown chunk
                        file.seek(SeekFrom::Current(data_size as i64))?;
                    }
                }
            } else {
                // Skip unknown chunk
                file.seek(SeekFrom::Current(chunk_size as i64))?;
            }
        }
    }
}

impl AudioCapture for FileAudioSource {
    fn start(&mut self) -> Result<()> {
        // No-op for file source
        Ok(())
    }

    fn read(&mut self, buffer: &mut [i16]) -> Result<usize> {
        let bytes_per_sample = (self.header.bits_per_sample / 8) as usize;
        let bytes_needed = buffer.len() * bytes_per_sample;
        let mut byte_buffer = vec![0u8; bytes_needed];

        let bytes_read = match self.file.read(&mut byte_buffer) {
            Ok(0) => {
                // EOF reached
                if self.loop_playback {
                    // Seek back to start of audio data and retry
                    self.file.seek(SeekFrom::Start(self.header.data_offset))?;
                    self.samples_read = 0;
                    self.file.read(&mut byte_buffer)?
                } else {
                    return Ok(0);
                }
            }
            Ok(n) => n,
            Err(e) => return Err(e.into()),
        };

        // Convert little-endian bytes to i16 samples
        let samples_read = bytes_read / bytes_per_sample;
        for i in 0..samples_read {
            let byte_offset = i * bytes_per_sample;
            buffer[i] = i16::from_le_bytes([
                byte_buffer[byte_offset],
                byte_buffer[byte_offset + 1],
            ]);
        }

        self.samples_read += samples_read;
        Ok(samples_read)
    }

    fn stop(&mut self) -> Result<()> {
        // No-op for file source
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::format;

    #[test]
    fn test_parse_wav_header() {
        use hound::{WavWriter, WavSpec};

        let spec = WavSpec {
            channels: 2,
            sample_rate: 48000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let path = "/tmp/test_parse_wav.wav";
        let mut writer = WavWriter::create(path, spec).unwrap();
        for _ in 0..1920 {
            writer.write_sample(0i16).unwrap();
        }
        writer.finalize().unwrap();

        let source = FileAudioSource::open(path).unwrap();
        assert_eq!(source.header.sample_rate, format::SAMPLE_RATE);
        assert_eq!(source.header.channels, format::CHANNELS);
        assert_eq!(source.header.bits_per_sample, 16);

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_read_audio_data() {
        use hound::{WavWriter, WavSpec};

        let spec = WavSpec {
            channels: 2,
            sample_rate: 48000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let path = "/tmp/test_read_audio.wav";
        let mut writer = WavWriter::create(path, spec).unwrap();

        // Write 2 frames (3840 samples = 1920 samples/frame * 2)
        for i in 0..3840 {
            writer.write_sample((i % 1000) as i16).unwrap();
        }
        writer.finalize().unwrap();

        let mut source = FileAudioSource::open(path).unwrap();
        let mut buffer = vec![0i16; 1920];

        // Read first frame
        let samples_read = source.read(&mut buffer).unwrap();
        assert_eq!(samples_read, 1920);
        assert_eq!(buffer[0], 0);
        assert_eq!(buffer[999], 999);

        // Read second frame
        let samples_read = source.read(&mut buffer).unwrap();
        assert_eq!(samples_read, 1920);
        assert_eq!(buffer[0], 920); // (1920 % 1000)

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_loop_playback() {
        use hound::{WavWriter, WavSpec};

        let spec = WavSpec {
            channels: 2,
            sample_rate: 48000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let path = "/tmp/test_loop_playback.wav";
        let mut writer = WavWriter::create(path, spec).unwrap();

        // Write 1 frame (1920 samples)
        for i in 0..1920 {
            writer.write_sample((i % 100) as i16).unwrap();
        }
        writer.finalize().unwrap();

        let mut source = FileAudioSource::open(path).unwrap().with_loop(true);
        let mut buffer = vec![0i16; 1920];

        // Read first time
        let samples_read = source.read(&mut buffer).unwrap();
        assert_eq!(samples_read, 1920);
        assert_eq!(buffer[0], 0);
        assert_eq!(buffer[99], 99);

        // Read second time - should loop back and read same data
        let samples_read = source.read(&mut buffer).unwrap();
        assert_eq!(samples_read, 1920);
        assert_eq!(buffer[0], 0);
        assert_eq!(buffer[99], 99);

        std::fs::remove_file(path).ok();
    }
}
