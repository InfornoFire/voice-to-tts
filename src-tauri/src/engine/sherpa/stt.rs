use std::path::Path;

use sherpa_onnx::{OnlineRecognizer, OnlineRecognizerConfig, OnlineStream};

use super::path_text;
use crate::engine::{INPUT_SAMPLE_RATE, SpeechError, Stt};

const MODEL_KIND: &str = "speech recognition";

/// Silence fed after an utterance, so the model produces its last words.
const TAIL_PADDING_SECONDS: f32 = 1.2;

/// The names of the files that make up a Zipformer transducer model.
#[derive(Debug, Clone, Copy)]
pub struct ZipformerFiles<'a> {
    pub encoder: &'a str,
    pub decoder: &'a str,
    pub joiner: &'a str,
    pub tokens: &'a str,
}

/// Streaming recognition with a Zipformer transducer model.
pub struct ZipformerStt {
    recognizer: OnlineRecognizer,
    stream: OnlineStream,
}

impl ZipformerStt {
    /// Loads the `files` in `model_dir`, using at most `threads` threads.
    pub fn load(model_dir: &Path, files: ZipformerFiles, threads: u8) -> Result<Self, SpeechError> {
        let file = |name: &str| path_text(&model_dir.join(name), MODEL_KIND).map(Some);

        let mut config = OnlineRecognizerConfig::default();
        config.model_config.transducer.encoder = file(files.encoder)?;
        config.model_config.transducer.decoder = file(files.decoder)?;
        config.model_config.transducer.joiner = file(files.joiner)?;
        config.model_config.tokens = file(files.tokens)?;
        config.model_config.num_threads = i32::from(threads);
        config.decoding_method = Some("greedy_search".to_owned());

        let recognizer = OnlineRecognizer::create(&config).ok_or(SpeechError::Load(MODEL_KIND))?;
        let stream = recognizer.create_stream();
        Ok(Self { recognizer, stream })
    }

    fn feed(&self, samples: &[f32]) {
        self.stream
            .accept_waveform(INPUT_SAMPLE_RATE as i32, samples);
    }

    fn decode(&self) -> String {
        while self.recognizer.is_ready(&self.stream) {
            self.recognizer.decode(&self.stream);
        }
        self.recognizer
            .get_result(&self.stream)
            .map(|result| result.text)
            .unwrap_or_default()
    }
}

impl Stt for ZipformerStt {
    fn accept(&mut self, samples: &[f32]) -> String {
        self.feed(samples);
        self.decode()
    }

    fn finish(&mut self) -> String {
        let padding = vec![0.0; (INPUT_SAMPLE_RATE as f32 * TAIL_PADDING_SECONDS) as usize];
        self.feed(&padding);
        self.stream.input_finished();
        let text = self.decode();
        self.recognizer.reset(&self.stream);
        text
    }
}
