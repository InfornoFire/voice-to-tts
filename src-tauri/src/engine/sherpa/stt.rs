use std::path::Path;

use sherpa_onnx::{OnlineRecognizer, OnlineRecognizerConfig, OnlineStream};

use super::path_text;
use crate::engine::{INPUT_SAMPLE_RATE, SpeechError, Stt};

const MODEL_KIND: &str = "speech recognition";

/// A streaming transducer model in a directory, and how it has to be fed.
#[derive(Debug, Clone, Copy)]
pub struct StreamingModel<'a> {
    pub encoder: &'a str,
    pub decoder: &'a str,
    pub joiner: &'a str,
    pub tokens: &'a str,
    pub tail_padding_seconds: f32,
}

/// Streaming recognition with a transducer model.
pub struct StreamingStt {
    recognizer: OnlineRecognizer,
    stream: OnlineStream,
    tail_padding_seconds: f32,
}

impl StreamingStt {
    /// Loads the `model` in `model_dir`, using at most `threads` threads.
    pub fn load(model_dir: &Path, model: StreamingModel, threads: u8) -> Result<Self, SpeechError> {
        let file = |name: &str| path_text(&model_dir.join(name), MODEL_KIND).map(Some);

        let mut config = OnlineRecognizerConfig::default();
        config.model_config.transducer.encoder = file(model.encoder)?;
        config.model_config.transducer.decoder = file(model.decoder)?;
        config.model_config.transducer.joiner = file(model.joiner)?;
        config.model_config.tokens = file(model.tokens)?;
        config.model_config.num_threads = i32::from(threads);
        config.decoding_method = Some("greedy_search".to_owned());

        let recognizer = OnlineRecognizer::create(&config).ok_or(SpeechError::Load(MODEL_KIND))?;
        let stream = recognizer.create_stream();
        Ok(Self {
            recognizer,
            stream,
            tail_padding_seconds: model.tail_padding_seconds,
        })
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

impl Stt for StreamingStt {
    fn accept(&mut self, samples: &[f32]) -> String {
        self.feed(samples);
        self.decode()
    }

    fn finish(&mut self) -> String {
        let padding = vec![0.0; (INPUT_SAMPLE_RATE as f32 * self.tail_padding_seconds) as usize];
        self.feed(&padding);
        self.stream.input_finished();
        let text = self.decode();
        self.stream = self.recognizer.create_stream();
        text
    }
}
