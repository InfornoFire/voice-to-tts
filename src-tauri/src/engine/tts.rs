use super::SpeechError;

/// Turns text into speech.
pub trait Tts: Send {
    /// The sample rate of the audio returned by [`Tts::synthesize`].
    fn sample_rate(&self) -> u32;

    /// Speaks `text` in `voice` at `speed` times the normal pace, returning mono samples.
    ///
    /// A voice the model does not have is spoken with its default voice.
    fn synthesize(&mut self, text: &str, voice: &str, speed: f32) -> Result<Vec<f32>, SpeechError>;
}
