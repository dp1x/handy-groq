//! Audio capture via cpal.
//!
//! Records from the default input device, stores 16 kHz mono f32 samples.
//! The caller sends these to Groq's API.

use anyhow::anyhow;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SizedSample};
use log::debug;
use std::sync::{Arc, Mutex};

/// Shared recording state. The cpal callback writes audio frames into the
/// buffer; the caller reads it out when the shortcut is released.
pub struct AudioRecorder {
    stream: Option<cpal::Stream>,
    buffer: Arc<Mutex<Vec<f32>>>,
    /// Rolling RMS amplitude values for waveform visualization (last ~50 chunks).
    levels: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            stream: None,
            buffer: Arc::new(Mutex::new(Vec::new())),
            levels: Arc::new(Mutex::new(Vec::new())),
            sample_rate: 16000,
        }
    }

    /// Start recording from the default input device.
    pub fn start(&mut self) -> anyhow::Result<()> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow!("No audio input device found"))?;

        let config = device
            .default_input_config()
            .map_err(|e| anyhow!("Failed to get input config: {}", e))?;

        self.sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;
        debug!(
            "Audio: device={}, sample_rate={}Hz, channels={}",
            device.name().unwrap_or_else(|_| "unknown".into()),
            self.sample_rate,
            channels
        );

        self.buffer.lock().unwrap().clear();
        self.levels.lock().unwrap().clear();

        let stream = match config.sample_format() {
            SampleFormat::I16 => build_stream::<i16>(
                &device,
                &config.into(),
                self.buffer.clone(),
                self.levels.clone(),
                channels,
            ),
            SampleFormat::U16 => build_stream::<u16>(
                &device,
                &config.into(),
                self.buffer.clone(),
                self.levels.clone(),
                channels,
            ),
            SampleFormat::F32 => build_stream::<f32>(
                &device,
                &config.into(),
                self.buffer.clone(),
                self.levels.clone(),
                channels,
            ),
            _ => return Err(anyhow!("Unsupported audio sample format")),
        }
        .map_err(|e| anyhow!("Failed to build audio stream: {}", e))?;

        stream
            .play()
            .map_err(|e| anyhow!("Failed to start audio stream: {}", e))?;

        self.stream = Some(stream);
        Ok(())
    }

    /// Stop recording and return (samples at device rate, sample_rate_hz).
    pub fn stop(&mut self) -> (Vec<f32>, u32) {
        self.stream.take();
        let samples = std::mem::take(&mut *self.buffer.lock().unwrap());
        self.levels.lock().unwrap().clear();
        (samples, self.sample_rate)
    }

    /// Returns the last N RMS amplitude values for waveform visualization.
    pub fn rms_levels(&self, n: usize) -> Vec<f32> {
        let lvls = self.levels.lock().unwrap();
        let start = lvls.len().saturating_sub(n);
        lvls[start..].to_vec()
    }
}

/// Build a cpal input stream that captures samples and converts to f32.
fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: Arc<Mutex<Vec<f32>>>,
    levels: Arc<Mutex<Vec<f32>>>,
    channels: usize,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: SizedSample + Send + 'static,
    f32: cpal::FromSample<T>,
{
    // For mono, we just take every sample. For multi-channel, we take the
    // first channel (left) by indexing into the interleaved frames.
    device.build_input_stream(
        config,
        move |data: &[T], _info: &cpal::InputCallbackInfo| {
            let mut buf = buffer.lock().unwrap();
            let mut sum_sq = 0.0f32;
            let mut count = 0;

            if channels == 1 {
                // Mono — direct copy
                for sample in data {
                    let f = sample.to_sample::<f32>();
                    sum_sq += f * f;
                    count += 1;
                    buf.push(f);
                }
            } else {
                // Multi-channel — take first channel from interleaved frames
                for frame in data.chunks(channels) {
                    if let Some(&first) = frame.first() {
                        let f = first.to_sample::<f32>();
                        sum_sq += f * f;
                        count += 1;
                        buf.push(f);
                    }
                }
            }

            let rms = if count > 0 {
                (sum_sq / count as f32).sqrt()
            } else {
                0.0
            };

            // Drop buffer lock before locking levels to avoid nested locks.
            drop(buf);

            let mut lvls = levels.lock().unwrap();
            lvls.push(rms);
            // Keep a rolling window of ~50 samples for the waveform.
            if lvls.len() > 50 {
                lvls.remove(0);
            }
        },
        move |err: cpal::StreamError| {
            log::error!("cpal stream error: {}", err);
        },
        None,
    )
}

/// Simple linear downsampling/upsampling to 16 kHz.
/// Uses nearest-neighbor — good enough for Whisper.
pub fn resample_to_16k(samples: &[f32], from_rate: u32) -> Vec<f32> {
    if from_rate == 16000 {
        return samples.to_vec();
    }

    let ratio = from_rate as f64 / 16000.0;
    let target_len = (samples.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(target_len);

    for i in 0..target_len {
        let src_idx = (i as f64 * ratio) as usize;
        if src_idx < samples.len() {
            out.push(samples[src_idx]);
        }
    }
    out
}

/// Encode 16 kHz mono f32 samples into a WAV file (bytes).
pub fn encode_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let mut buf = Vec::new();
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    {
        let mut writer = hound::WavWriter::new(std::io::Cursor::new(&mut buf), spec)
            .expect("failed to create WAV writer");

        for &s in samples {
            let clipped = s.clamp(-1.0, 1.0);
            writer.write_sample(clipped).ok();
        }
        writer.flush().ok();
    }

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_wav_empty() {
        let wav = encode_wav(&[], 16000);
        assert!(wav.len() >= 44);
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
    }

    #[test]
    fn test_encode_wav_silence() {
        let samples = vec![0.0f32; 16000]; // 1 second of silence
        let wav = encode_wav(&samples, 16000);
        assert!(wav.len() > 44); // header + data
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
    }

    #[test]
    fn test_resample_identity() {
        let samples: Vec<f32> = (0..16000)
            .map(|i| (i as f32 / 16000.0) * 2.0 - 1.0)
            .collect();
        let resampled = resample_to_16k(&samples, 16000);
        assert_eq!(resampled.len(), samples.len());
    }

    #[test]
    fn test_resample_down() {
        let samples = vec![0.5f32; 48000]; // 1 second at 48 kHz
        let resampled = resample_to_16k(&samples, 48000);
        assert_eq!(resampled.len(), 16000); // 1 second at 16 kHz
    }

    #[test]
    fn test_resample_up() {
        let samples = vec![0.5f32; 8000]; // 1 second at 8 kHz
        let resampled = resample_to_16k(&samples, 8000);
        assert_eq!(resampled.len(), 16000); // 1 second at 16 kHz
    }
}

/// Enumerate available input (recording) devices by name.
pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .into_iter()
        .flatten()
        .filter_map(|d| d.name().ok())
        .collect()
}

/// Enumerate available output (playback) devices by name.
pub fn list_output_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.output_devices()
        .into_iter()
        .flatten()
        .filter_map(|d| d.name().ok())
        .collect()
}
