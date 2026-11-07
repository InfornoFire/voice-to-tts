use super::SpeechError;

/// Turns text into speech.
pub trait Tts: Send {
    /// The sample rate of the audio returned by [`Tts::synthesize`].
    fn sample_rate(&self) -> u32;

    /// Speaks `text` at `speed` times the normal pace, returning mono samples.
    fn synthesize(&mut self, text: &str, speed: f32) -> Result<Vec<f32>, SpeechError>;
}
