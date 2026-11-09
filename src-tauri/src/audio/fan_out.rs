//! Plays speech through several outputs at once.

use super::AudioOutput;
use super::devices::AudioError;

/// Passes every clip to each of its outputs.
pub struct FanOutOutput {
    outputs: Vec<Box<dyn AudioOutput>>,
}

impl FanOutOutput {
    pub fn new(outputs: Vec<Box<dyn AudioOutput>>) -> Self {
        Self { outputs }
    }
}

impl AudioOutput for FanOutOutput {
    /// Stops at the first output that fails.
    fn play(&mut self, samples: &[f32], sample_rate: u32) -> Result<(), AudioError> {
        self.outputs
            .iter_mut()
            .try_for_each(|output| output.play(samples, sample_rate))
    }

    fn is_idle(&self) -> bool {
        self.outputs.iter().all(|output| output.is_idle())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    use super::*;

    struct Recorder {
        played: Arc<Mutex<Vec<Vec<f32>>>>,
        idle: Arc<AtomicBool>,
    }

    impl AudioOutput for Recorder {
        fn play(&mut self, samples: &[f32], _sample_rate: u32) -> Result<(), AudioError> {
            self.played.lock().unwrap().push(samples.to_vec());
            Ok(())
        }

        fn is_idle(&self) -> bool {
            self.idle.load(Ordering::Relaxed)
        }
    }

    #[test]
    fn every_output_gets_the_clip_and_all_must_be_idle() {
        let played = Arc::new(Mutex::new(Vec::new()));
        let first_idle = Arc::new(AtomicBool::new(true));
        let second_idle = Arc::new(AtomicBool::new(false));
        let recorder = |idle: &Arc<AtomicBool>| -> Box<dyn AudioOutput> {
            Box::new(Recorder {
                played: Arc::clone(&played),
                idle: Arc::clone(idle),
            })
        };
        let mut fan_out = FanOutOutput::new(vec![recorder(&first_idle), recorder(&second_idle)]);

        fan_out.play(&[0.5, 0.25], 16_000).unwrap();

        assert_eq!(*played.lock().unwrap(), [vec![0.5, 0.25], vec![0.5, 0.25]]);
        assert!(!fan_out.is_idle());
        second_idle.store(true, Ordering::Relaxed);
        assert!(fan_out.is_idle());
    }
}
