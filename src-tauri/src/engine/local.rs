//! The audio devices and speech models of this machine.

use std::path::PathBuf;

use super::{
    INPUT_SAMPLE_RATE, Models, OutputTarget, Resources, SherpaTts, SileroVad, SpeechError,
    StreamingModel, StreamingStt,
};
use crate::audio::devices::{AudioError, is_cable_installed, is_virtual_microphone};
use crate::audio::microphone::Microphone;
use crate::audio::playback::Playback;
use crate::audio::{AudioInput, AudioOutput};
use crate::config::Config;
use crate::models::Voice;

const VAD_FILE: &str = "silero_vad.onnx";
const LEXICON_FILE: &str = "lexicon-en-us.txt";

/// The directories of the speech models within the models directory.
const RECOGNIZER_DIR: &str = "sherpa-onnx-nemotron-speech-streaming-en-0.6b-560ms-int8";
const VOICE_DIR: &str = "kitten-nano-en-v0_8-int8";

/// The files of the speech recognition model, and how it has to be fed.
const RECOGNIZER_MODEL: StreamingModel = StreamingModel {
    encoder: "encoder.int8.onnx",
    decoder: "decoder.int8.onnx",
    joiner: "joiner.int8.onnx",
    tokens: "tokens.txt",
    tail_padding_seconds: 0.3,
};

/// The audio devices of this machine and the models in a directory.
pub struct LocalResources {
    models_dir: PathBuf,
    voices: Vec<Voice>,
}

impl LocalResources {
    /// The first of the `voices` is the default.
    pub fn new(models_dir: PathBuf, voices: Vec<Voice>) -> Self {
        Self { models_dir, voices }
    }
}

impl Resources for LocalResources {
    fn is_cable_installed(&self) -> bool {
        is_cable_installed()
    }

    fn is_virtual_microphone(&self, device_id: Option<&str>) -> bool {
        is_virtual_microphone(device_id)
    }

    fn open_input(&self, device_id: Option<&str>) -> Result<Box<dyn AudioInput>, AudioError> {
        Ok(Box::new(Microphone::open(device_id, INPUT_SAMPLE_RATE)?))
    }

    fn open_output(&self, target: OutputTarget) -> Result<Box<dyn AudioOutput>, AudioError> {
        let playback = match target {
            OutputTarget::VirtualMicrophone => Playback::virtual_microphone()?,
            OutputTarget::DefaultOutput => Playback::default_output()?,
        };
        Ok(Box::new(playback))
    }

    fn load_models(&self, config: &Config) -> Result<Models, SpeechError> {
        let models = &self.models_dir;
        let recognizer = models.join(RECOGNIZER_DIR);
        let voice = models.join(VOICE_DIR);
        let lexicon = models.join(LEXICON_FILE);
        Ok(Models {
            vad: Box::new(SileroVad::load(&models.join(VAD_FILE))?),
            stt: Box::new(StreamingStt::load(
                &recognizer,
                RECOGNIZER_MODEL,
                config.threads,
            )?),
            tts: Box::new(SherpaTts::kitten(
                &voice,
                &lexicon,
                self.voices.clone(),
                config.threads,
            )?),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

    use super::*;
    use crate::audio::resample::resample_clip;
    use crate::download::HttpFetch;
    use crate::engine::{Heard, Listener};
    use crate::models::{Manifest, ModelManager, ModelsState};

    fn models_dir() -> PathBuf {
        let local_data = std::env::var_os("LOCALAPPDATA").expect("LOCALAPPDATA should be set");
        PathBuf::from(local_data)
            .join("com.inforno.voice-to-tts")
            .join("models")
    }

    fn cpu_seconds() -> f64 {
        let (mut creation, mut exit, mut kernel, mut user) = Default::default();
        let ticks = |time: FILETIME| {
            (u64::from(time.dwHighDateTime) << 32 | u64::from(time.dwLowDateTime)) as f64
        };
        // The pointers refer to locals and the handle is the current process.
        unsafe {
            GetProcessTimes(
                GetCurrentProcess(),
                &mut creation,
                &mut exit,
                &mut kernel,
                &mut user,
            )
            .unwrap();
        }
        (ticks(kernel) + ticks(user)) / 10_000_000.0
    }

    fn working_set_megabytes() -> f64 {
        let mut counters = PROCESS_MEMORY_COUNTERS::default();
        // The pointer refers to a local and the handle is the current process.
        unsafe {
            GetProcessMemoryInfo(
                GetCurrentProcess(),
                &mut counters,
                size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            )
            .unwrap();
        }
        counters.WorkingSetSize as f64 / 1_048_576.0
    }

    fn timed<T>(label: &str, work: impl FnOnce() -> T) -> T {
        let (start, cpu_start) = (Instant::now(), cpu_seconds());
        let value = work();
        println!(
            "{label}: {:.2} s wall, {:.2} s cpu, {:.0} MB in use",
            start.elapsed().as_secs_f64(),
            cpu_seconds() - cpu_start,
            working_set_megabytes()
        );
        value
    }

    #[test]
    #[ignore = "downloads the models to the models directory"]
    fn downloaded_models_recognize_their_own_speech() {
        let manager = ModelManager::new(
            models_dir(),
            Manifest::bundled(),
            Box::new(HttpFetch::new()),
            Box::new(|_| {}),
        );
        assert_eq!(manager.download(), ModelsState::Ready);
        let mut models = LocalResources::new(manager.dir().to_owned(), manager.voices().to_vec())
            .load_models(&Config::default())
            .unwrap();

        let sentence = "Hello there, this is a test of the voice proxy.";
        let first_voice = manager.voices().first().unwrap();
        let last_voice = manager.voices().last().unwrap();
        let speech = models
            .tts
            .synthesize(sentence, &first_voice.id, 1.0)
            .unwrap();
        let other_speech = models
            .tts
            .synthesize(sentence, &last_voice.id, 1.0)
            .unwrap();
        assert_ne!(speech, other_speech);
        let speech = resample_clip(&speech, models.tts.sample_rate(), INPUT_SAMPLE_RATE).unwrap();
        let mut audio = vec![0.0; INPUT_SAMPLE_RATE as usize];
        audio.extend(speech);
        audio.extend(vec![0.0; 2 * INPUT_SAMPLE_RATE as usize]);

        let mut listener = Listener::new(models.vad, models.stt);
        let heard: Vec<_> = audio
            .chunks(INPUT_SAMPLE_RATE as usize / 50)
            .filter_map(|chunk| listener.accept(chunk))
            .collect();

        let Some(Heard::Final(text)) = heard.last() else {
            panic!("no utterance ended: {heard:?}");
        };
        let text = text.to_lowercase();
        assert!(text.contains("test") && text.contains("voice"), "{text}");
    }

    #[test]
    #[ignore = "needs the models downloaded to the models directory"]
    fn partial_text_while_speech_is_recognized() {
        const STEP_SAMPLES: usize = INPUT_SAMPLE_RATE as usize / 50;
        const SENTENCES: [&str; 3] = [
            "Hello there, this is a test of the voice proxy.",
            "I paid twenty dollars for it, but the GPU was not working at all.",
            "We could start the meeting now, or we could wait a few more minutes for everyone.",
        ];

        let voices = Manifest::bundled().voices;
        let mut models = LocalResources::new(models_dir(), voices.clone())
            .load_models(&Config::default())
            .unwrap();
        let seconds = |samples: usize| samples as f32 / INPUT_SAMPLE_RATE as f32;
        let silence = |length: f32| vec![0.0; (INPUT_SAMPLE_RATE as f32 * length) as usize];

        let mut audio = Vec::new();
        for sentence in SENTENCES {
            audio.extend(silence(1.0));
            let speech = models.tts.synthesize(sentence, &voices[0].id, 1.0).unwrap();
            let speech =
                resample_clip(&speech, models.tts.sample_rate(), INPUT_SAMPLE_RATE).unwrap();
            println!(
                "speech from {:.2} s to {:.2} s: {sentence}",
                seconds(audio.len()),
                seconds(audio.len() + speech.len())
            );
            audio.extend(speech);
            audio.extend(silence(1.5));
        }

        let mut listener = Listener::new(models.vad, models.stt);
        let mut earlier = Vec::<String>::new();
        let mut changed_words = 0;
        let mut updates = 0;
        for (index, chunk) in audio.chunks(STEP_SAMPLES).enumerate() {
            let now = seconds((index + 1) * STEP_SAMPLES);
            match listener.accept(chunk) {
                Some(Heard::Partial(text)) => {
                    let words: Vec<String> = text.split(' ').map(str::to_owned).collect();
                    let settled = earlier.len().saturating_sub(1);
                    let changed = (0..settled)
                        .filter(|&position| words.get(position) != earlier.get(position))
                        .count();
                    changed_words += changed;
                    updates += 1;
                    println!("{now:.2} s, {changed} settled words changed: {text}");
                    earlier = words;
                }
                Some(Heard::Final(text)) => {
                    println!("{now:.2} s, final: {text}");
                    earlier.clear();
                }
                None => {}
            }
        }
        println!("{changed_words} settled words changed over {updates} updates");
    }

    #[test]
    #[ignore = "needs the models downloaded to the models directory"]
    fn idle_cost_of_the_loaded_models() {
        const POLL_SAMPLES: usize = 320;
        const SECONDS: usize = 60;

        let before = working_set_megabytes();
        let resources = LocalResources::new(models_dir(), Manifest::bundled().voices);
        let models = timed("load all the models", || {
            resources.load_models(&Config::default()).unwrap()
        });
        let mut listener = Listener::new(models.vad, models.stt);
        let silence = vec![0.0; POLL_SAMPLES];

        // The app hands the listener 20 ms of audio at a time.
        timed(&format!("{SECONDS} s of silence while listening"), || {
            for _ in 0..SECONDS * 50 {
                listener.accept(&silence);
            }
        });

        drop(listener);
        drop(models.tts);
        println!(
            "memory: {before:.0} MB before loading, {:.0} MB after unloading",
            working_set_megabytes()
        );
    }
}
