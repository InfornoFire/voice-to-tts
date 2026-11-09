//! Captures audio from a microphone.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{Device, Sample, SampleFormat, SizedSample, Stream, StreamConfig};
use ringbuf::traits::{Producer, Split};
use ringbuf::{HeapCons, HeapProd, HeapRb};

use super::devices::AudioError;
use super::{flag_failure, unsupported_sample_format};

const BUFFER_SECONDS: usize = 2;

/// A running capture of a microphone. Dropping it stops the capture.
pub struct Capture {
    _stream: Stream,
    sample_rate: u32,
    failed: Arc<AtomicBool>,
}

impl Capture {
    /// Starts capturing from `device`, returning mono samples at the device sample rate.
    ///
    /// Samples are dropped when the buffer is not read within [`BUFFER_SECONDS`] seconds.
    pub fn start(device: &Device) -> Result<(Self, HeapCons<f32>), AudioError> {
        let supported = device.default_input_config().map_err(AudioError::OpenMic)?;
        let config = supported.config();
        let capacity = config.sample_rate as usize * BUFFER_SECONDS;
        let (producer, consumer) = HeapRb::<f32>::new(capacity).split();
        let failed = Arc::new(AtomicBool::new(false));

        let stream = match supported.sample_format() {
            SampleFormat::F32 => build_stream::<f32>(device, &config, producer, &failed),
            SampleFormat::I16 => build_stream::<i16>(device, &config, producer, &failed),
            SampleFormat::I32 => build_stream::<i32>(device, &config, producer, &failed),
            _ => Err(unsupported_sample_format()),
        }
        .map_err(AudioError::OpenMic)?;
        stream.play().map_err(AudioError::OpenMic)?;

        let capture = Self {
            _stream: stream,
            sample_rate: config.sample_rate,
            failed,
        };
        Ok((capture, consumer))
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Whether the microphone went away.
    pub fn has_failed(&self) -> bool {
        self.failed.load(Ordering::Relaxed)
    }
}

fn build_stream<T>(
    device: &Device,
    config: &StreamConfig,
    mut producer: HeapProd<f32>,
    failed: &Arc<AtomicBool>,
) -> Result<Stream, cpal::Error>
where
    T: SizedSample,
    f32: cpal::FromSample<T>,
{
    let channels = usize::from(config.channels);
    device.build_input_stream(
        *config,
        move |data: &[T], _| push_mono(&mut producer, data, channels),
        flag_failure(failed),
        None,
    )
}

/// Averages the channels of each frame, dropping what does not fit in the buffer.
fn push_mono<T>(producer: &mut impl Producer<Item = f32>, data: &[T], channels: usize)
where
    T: SizedSample,
    f32: cpal::FromSample<T>,
{
    producer.push_iter(data.chunks_exact(channels).map(|frame| {
        let sum: f32 = frame.iter().map(|&sample| f32::from_sample(sample)).sum();
        sum / channels as f32
    }));
}

#[cfg(test)]
mod tests {
    use ringbuf::traits::{Consumer, Observer};

    use super::*;

    #[test]
    fn frames_are_averaged_and_overflow_is_dropped() {
        let (mut producer, mut consumer) = HeapRb::<f32>::new(2).split();

        push_mono(&mut producer, &[0.5_f32, -0.5, 1.0, 0.0, 0.25, 0.75], 2);

        assert_eq!(consumer.pop_iter().collect::<Vec<_>>(), [0.0, 0.5]);
        assert!(consumer.is_empty());
    }
}
