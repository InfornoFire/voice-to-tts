use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{Device, FromSample, SampleFormat, SizedSample, Stream, StreamConfig};
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::{HeapCons, HeapProd, HeapRb};

use super::devices::{AudioError, find_cable_output};
use super::resample::resample_clip;
use super::{AudioOutput, is_fatal};

const BUFFER_SECONDS: usize = 30;
const FULL_BUFFER_POLL: Duration = Duration::from_millis(10);

/// A running output stream to the virtual microphone. Dropping it stops the stream.
pub struct CableOutput {
    _stream: Stream,
    producer: HeapProd<f32>,
    sample_rate: u32,
    failed: Arc<AtomicBool>,
}

impl CableOutput {
    /// Opens the virtual microphone, the only output stream the app creates.
    pub fn open() -> Result<Self, AudioError> {
        let device = find_cable_output()?;
        let supported = device
            .default_output_config()
            .map_err(AudioError::OpenCable)?;
        let config = supported.config();
        let capacity = config.sample_rate as usize * BUFFER_SECONDS;
        let (producer, consumer) = HeapRb::<f32>::new(capacity).split();
        let failed = Arc::new(AtomicBool::new(false));

        let stream = match supported.sample_format() {
            SampleFormat::F32 => build_stream::<f32>(&device, &config, consumer, &failed),
            SampleFormat::I16 => build_stream::<i16>(&device, &config, consumer, &failed),
            SampleFormat::I32 => build_stream::<i32>(&device, &config, consumer, &failed),
            _ => Err(cpal::Error::with_message(
                cpal::ErrorKind::UnsupportedConfig,
                "unsupported sample format",
            )),
        }
        .map_err(AudioError::OpenCable)?;
        stream.play().map_err(AudioError::OpenCable)?;

        Ok(Self {
            _stream: stream,
            producer,
            sample_rate: config.sample_rate,
            failed,
        })
    }
}

impl AudioOutput for CableOutput {
    /// Waits while the buffer is full, and fails when the virtual microphone is lost.
    fn play(&mut self, samples: &[f32], sample_rate: u32) -> Result<(), AudioError> {
        let converted = resample_clip(samples, sample_rate, self.sample_rate)?;
        let mut remaining = converted.as_slice();
        while !remaining.is_empty() {
            if self.failed.load(Ordering::Relaxed) {
                return Err(AudioError::CableMissing);
            }
            remaining = &remaining[self.producer.push_slice(remaining)..];
            if !remaining.is_empty() {
                thread::sleep(FULL_BUFFER_POLL);
            }
        }
        Ok(())
    }

    fn is_idle(&self) -> bool {
        self.producer.is_empty()
    }
}

fn build_stream<T>(
    device: &Device,
    config: &StreamConfig,
    mut consumer: HeapCons<f32>,
    failed: &Arc<AtomicBool>,
) -> Result<Stream, cpal::Error>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = usize::from(config.channels);
    let failed = Arc::clone(failed);
    device.build_output_stream(
        *config,
        move |data: &mut [T], _| fill_output(data, channels, &mut consumer),
        move |error| {
            if is_fatal(&error) {
                failed.store(true, Ordering::Relaxed);
            }
        },
        None,
    )
}

/// Writes each queued sample to every channel of a frame, and silence when the queue runs dry.
fn fill_output<T>(data: &mut [T], channels: usize, consumer: &mut impl Consumer<Item = f32>)
where
    T: SizedSample + FromSample<f32>,
{
    for frame in data.chunks_exact_mut(channels) {
        frame.fill(T::from_sample(consumer.try_pop().unwrap_or(0.0)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_fill_every_channel_and_gaps_are_silent() {
        let (mut producer, mut consumer) = HeapRb::<f32>::new(4).split();
        producer.push_slice(&[0.5, 0.25]);
        let mut output = [1.0_f32; 6];

        fill_output(&mut output, 2, &mut consumer);

        assert_eq!(output, [0.5, 0.5, 0.25, 0.25, 0.0, 0.0]);
    }
}
