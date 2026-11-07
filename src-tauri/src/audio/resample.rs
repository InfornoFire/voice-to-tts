use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};

use super::devices::AudioError;

const CHUNK_FRAMES: usize = 1024;
const SIZES_MATCH: &str = "the buffers are sized from what the resampler asks for";

/// Converts a stream of mono samples to another sample rate.
pub struct StreamResampler {
    inner: Fft<f32>,
    pending: Vec<f32>,
    converted: Vec<f32>,
}

impl StreamResampler {
    pub fn new(from_rate: u32, to_rate: u32) -> Result<Self, AudioError> {
        let inner = Fft::new(
            from_rate as usize,
            to_rate as usize,
            CHUNK_FRAMES,
            1,
            FixedSync::Input,
        )?;
        let converted = vec![0.0; inner.output_frames_max()];
        Ok(Self {
            inner,
            pending: Vec::new(),
            converted,
        })
    }

    /// Appends the converted `input` to `output`, keeping any remainder for the next call.
    pub fn process(&mut self, input: &[f32], output: &mut Vec<f32>) {
        self.pending.extend_from_slice(input);

        let mut consumed = 0;
        loop {
            let needed = self.inner.input_frames_next();
            let Some(chunk) = self.pending.get(consumed..consumed + needed) else {
                break;
            };
            let chunk = InterleavedSlice::new(chunk, 1, needed).expect(SIZES_MATCH);
            let capacity = self.converted.len();
            let mut converted =
                InterleavedSlice::new_mut(&mut self.converted, 1, capacity).expect(SIZES_MATCH);
            let (_, produced) = self
                .inner
                .process_into_buffer(&chunk, &mut converted, None)
                .expect(SIZES_MATCH);
            output.extend_from_slice(&self.converted[..produced]);
            consumed += needed;
        }
        self.pending.drain(..consumed);
    }
}

/// Converts a whole clip, such as one synthesized sentence.
pub fn resample_clip(
    samples: &[f32],
    from_rate: u32,
    to_rate: u32,
) -> Result<Vec<f32>, AudioError> {
    if from_rate == to_rate || samples.is_empty() {
        return Ok(samples.to_vec());
    }

    let mut resampler = Fft::<f32>::new(
        from_rate as usize,
        to_rate as usize,
        CHUNK_FRAMES,
        1,
        FixedSync::Input,
    )?;
    let capacity = resampler.process_all_needed_output_len(samples.len());
    let mut converted = vec![0.0; capacity];

    let input = InterleavedSlice::new(samples, 1, samples.len()).expect(SIZES_MATCH);
    let mut output = InterleavedSlice::new_mut(&mut converted, 1, capacity).expect(SIZES_MATCH);
    let (_, produced) = resampler
        .process_all_into_buffer(&input, &mut output, samples.len(), None)
        .expect(SIZES_MATCH);
    converted.truncate(produced);
    Ok(converted)
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;

    use super::*;

    fn sine(rate: u32, seconds: u32) -> Vec<f32> {
        (0..rate * seconds)
            .map(|index| 0.5 * (TAU * 440.0 * index as f32 / rate as f32).sin())
            .collect()
    }

    fn peak(samples: &[f32]) -> f32 {
        samples
            .iter()
            .fold(0.0, |peak, sample| peak.max(sample.abs()))
    }

    #[test]
    fn clips_keep_their_duration_and_level_across_rates() {
        for (from_rate, to_rate) in [(22_050, 48_000), (24_000, 44_100), (48_000, 16_000)] {
            let converted = resample_clip(&sine(from_rate, 1), from_rate, to_rate).unwrap();

            let expected_length = f64::from(to_rate);
            let length_error = (converted.len() as f64 - expected_length).abs() / expected_length;
            assert!(
                length_error < 0.02,
                "{from_rate} to {to_rate}: {length_error}"
            );
            assert!(
                (peak(&converted) - 0.5).abs() < 0.05,
                "{from_rate} to {to_rate}"
            );
        }
    }

    #[test]
    fn streams_convert_the_same_however_the_input_is_split() {
        let input = sine(48_000, 1);
        let mut whole = Vec::new();
        StreamResampler::new(48_000, 16_000)
            .unwrap()
            .process(&input, &mut whole);

        let mut split = Vec::new();
        let mut resampler = StreamResampler::new(48_000, 16_000).unwrap();
        for piece in input.chunks(333) {
            resampler.process(piece, &mut split);
        }

        assert_eq!(whole, split);
        assert!(whole.len() > 15_000);
    }
}
