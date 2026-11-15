//! Speed, pitch and volume of the voice.
//!
//! The speech model only takes a speed, so pitch and volume are applied to the finished clip.

const MIN_SYNTHESIS_SPEED: f32 = 0.3;
const MAX_SYNTHESIS_SPEED: f32 = 3.0;

/// How the voice should sound.
#[derive(Debug)]
pub struct Tone {
    speed: f32,
    pitch_ratio: f32,
    volume: f32,
}

impl Tone {
    /// Creates a tone with `speed` and `volume` as multipliers, and a pitch of `semitones` above
    /// the normal voice.
    pub fn new(speed: f32, semitones: f32, volume: f32) -> Self {
        Self {
            speed,
            pitch_ratio: 2.0_f32.powf(semitones / 12.0),
            volume,
        }
    }

    /// The speed to ask the model for.
    ///
    /// A clip played faster to raise its pitch also gets shorter, so the model speaks slower to
    /// make up for it.
    pub fn synthesis_speed(&self) -> f32 {
        (self.speed / self.pitch_ratio).clamp(MIN_SYNTHESIS_SPEED, MAX_SYNTHESIS_SPEED)
    }

    /// The sample rate to play a clip at. Playing it faster raises its pitch.
    pub fn playback_rate(&self, sample_rate: u32) -> u32 {
        ((sample_rate as f32 * self.pitch_ratio).round() as u32).max(1)
    }

    /// Scales `samples` by the volume, without letting them go past full scale.
    pub fn apply_volume(&self, samples: &mut [f32]) {
        for sample in samples {
            *sample = (*sample * self.volume).clamp(-1.0, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pitch_changes_the_rate_and_the_synthesis_speed_makes_up_for_it() {
        let plain = Tone::new(2.0, 0.0, 1.0);
        let octave_up = Tone::new(2.0, 12.0, 1.0);
        let octave_down = Tone::new(2.0, -12.0, 1.0);

        assert_eq!(plain.playback_rate(24_000), 24_000);
        assert_eq!(plain.synthesis_speed(), 2.0);
        assert_eq!(octave_up.playback_rate(24_000), 48_000);
        assert_eq!(octave_up.synthesis_speed(), 1.0);
        assert_eq!(octave_down.playback_rate(24_000), 12_000);
        assert_eq!(octave_down.synthesis_speed(), MAX_SYNTHESIS_SPEED);
    }

    #[test]
    fn volume_scales_samples_and_stops_at_full_scale() {
        let mut samples = [0.25, -0.5, 0.75];

        Tone::new(1.0, 0.0, 1.5).apply_volume(&mut samples);

        assert_eq!(samples, [0.375, -0.75, 1.0]);
    }
}
