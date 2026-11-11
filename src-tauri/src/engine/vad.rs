//! What the engine needs from a voice detector.

/// Tells speech from silence in 16 kHz mono audio.
pub trait Vad: Send {
    /// Feeds `samples` and returns whether speech is going on after them.
    fn accept(&mut self, samples: &[f32]) -> bool;

    /// Forgets the audio heard so far, so the next speech starts afresh.
    fn reset(&mut self);
}
