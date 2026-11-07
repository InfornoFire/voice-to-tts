/// Recognizes speech in 16 kHz mono audio, one utterance at a time.
pub trait Stt: Send {
    /// Feeds `samples` of the current utterance and returns the text recognized so far.
    fn accept(&mut self, samples: &[f32]) -> String;

    /// Ends the current utterance, returning its final text, and readies the next one.
    fn finish(&mut self) -> String;
}
