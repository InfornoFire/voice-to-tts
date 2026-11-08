use ringbuf::HeapCons;
use ringbuf::traits::Consumer;

use super::AudioInput;
use super::capture::Capture;
use super::devices::{AudioError, find_input_device};
use super::resample::StreamResampler;

/// A running microphone delivering mono audio at a chosen sample rate.
pub struct Microphone {
    capture: Capture,
    captured: HeapCons<f32>,
    resampler: StreamResampler,
    device_rate_samples: Vec<f32>,
}

impl Microphone {
    /// Opens the microphone with `device_id`, or the system default when `None`.
    pub fn open(device_id: Option<&str>, sample_rate: u32) -> Result<Self, AudioError> {
        let device = find_input_device(device_id)?;
        let (capture, captured) = Capture::start(&device)?;
        let resampler = StreamResampler::new(capture.sample_rate(), sample_rate)?;
        Ok(Self {
            capture,
            captured,
            resampler,
            device_rate_samples: Vec::new(),
        })
    }
}

impl AudioInput for Microphone {
    fn read(&mut self, samples: &mut Vec<f32>) {
        self.device_rate_samples.clear();
        self.device_rate_samples.extend(self.captured.pop_iter());
        self.resampler.process(&self.device_rate_samples, samples);
    }

    fn has_failed(&self) -> bool {
        self.capture.has_failed()
    }
}
