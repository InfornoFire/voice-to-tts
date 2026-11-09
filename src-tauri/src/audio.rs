//! Audio devices and streams.

pub mod capture;
pub mod devices;
pub mod fan_out;
pub mod microphone;
pub mod playback;
pub mod resample;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use devices::AudioError;

/// A microphone delivering mono audio.
pub trait AudioInput: Send {
    /// Appends the samples captured since the previous call to `samples`.
    fn read(&mut self, samples: &mut Vec<f32>);

    /// Whether the microphone went away.
    fn has_failed(&self) -> bool;
}

/// Where synthesized speech is played.
pub trait AudioOutput: Send {
    /// Queues mono `samples` recorded at `sample_rate`.
    fn play(&mut self, samples: &[f32], sample_rate: u32) -> Result<(), AudioError>;

    /// Whether everything queued has been handed to the device.
    fn is_idle(&self) -> bool;
}

/// The error callback of a stream, which sets `failed` when an error means the stream is gone.
fn flag_failure(failed: &Arc<AtomicBool>) -> impl FnMut(cpal::Error) + Send + 'static {
    use cpal::ErrorKind::{DeviceNotAvailable, HostUnavailable, StreamInvalidated};
    let failed = Arc::clone(failed);
    move |error| {
        if matches!(
            error.kind(),
            DeviceNotAvailable | HostUnavailable | StreamInvalidated
        ) {
            failed.store(true, Ordering::Relaxed);
        }
    }
}

/// The error for a sample format that the streams cannot convert.
fn unsupported_sample_format() -> cpal::Error {
    cpal::Error::with_message(
        cpal::ErrorKind::UnsupportedConfig,
        "unsupported sample format",
    )
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;
    use std::thread;
    use std::time::Duration;

    use cpal::traits::{DeviceTrait, HostTrait};
    use ringbuf::traits::Consumer;

    use super::AudioOutput;
    use super::capture::Capture;
    use super::devices::is_cable;
    use super::playback::Playback;

    #[test]
    #[ignore = "needs VB-CABLE and plays a tone into it"]
    fn tone_played_to_the_cable_comes_out_of_its_output() {
        let cable_output = cpal::default_host()
            .input_devices()
            .unwrap()
            .find(|device| device.description().is_ok_and(|d| is_cable(d.name())))
            .expect("the virtual microphone should be installed");
        let (capture, mut captured) = Capture::start(&cable_output).unwrap();
        let mut output = Playback::virtual_microphone().unwrap();

        let tone: Vec<f32> = (0..22_050)
            .map(|index| 0.5 * (TAU * 440.0 * index as f32 / 22_050.0).sin())
            .collect();
        output.play(&tone, 22_050).unwrap();
        while !output.is_idle() {
            thread::sleep(Duration::from_millis(20));
        }
        thread::sleep(Duration::from_millis(500));

        let peak = captured
            .pop_iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
        assert!(!capture.has_failed());
        assert!(peak > 0.3, "peak was {peak}");
    }

    #[test]
    #[ignore = "needs VB-CABLE and another audio output, and plays silence through it"]
    fn listening_to_the_cable_plays_through_an_output_other_than_the_cable() {
        use super::devices::{find_default_output, is_virtual_microphone, list_input_devices};

        let cable_output = list_input_devices()
            .unwrap()
            .into_iter()
            .find(|device| device.is_cable)
            .expect("the virtual microphone should be installed");
        assert!(is_virtual_microphone(Some(&cable_output.id)));

        let output = find_default_output().unwrap();
        assert!(output.description().is_ok_and(|d| !is_cable(d.name())));

        let mut playback = Playback::default_output().unwrap();
        playback.play(&[0.0; 2_205], 22_050).unwrap();
        while !playback.is_idle() {
            thread::sleep(Duration::from_millis(20));
        }
    }
}
