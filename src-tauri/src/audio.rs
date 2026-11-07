//! Audio devices and streams.

pub mod capture;
pub mod devices;
pub mod playback;
pub mod resample;

/// Whether a stream error means the stream is gone.
fn is_fatal(error: &cpal::Error) -> bool {
    use cpal::ErrorKind::{DeviceNotAvailable, HostUnavailable, StreamInvalidated};
    matches!(
        error.kind(),
        DeviceNotAvailable | HostUnavailable | StreamInvalidated
    )
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;
    use std::thread;
    use std::time::Duration;

    use cpal::traits::{DeviceTrait, HostTrait};
    use ringbuf::traits::Consumer;

    use super::capture::Capture;
    use super::devices::is_cable;
    use super::playback::CableOutput;

    #[test]
    #[ignore = "needs VB-CABLE and plays a tone into it"]
    fn tone_played_to_the_cable_comes_out_of_its_output() {
        let cable_output = cpal::default_host()
            .input_devices()
            .unwrap()
            .find(|device| device.description().is_ok_and(|d| is_cable(d.name())))
            .expect("the virtual microphone should be installed");
        let (capture, mut captured) = Capture::start(&cable_output).unwrap();
        let mut output = CableOutput::open().unwrap();

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
}
