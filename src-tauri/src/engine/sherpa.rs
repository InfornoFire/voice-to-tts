//! Speech models run by sherpa-onnx.

mod stt;
mod tts;
mod vad;

use std::path::Path;

use super::SpeechError;

pub use stt::{ZipformerFiles, ZipformerStt};
pub use tts::SherpaTts;
pub use vad::SileroVad;

/// The text of `path`
fn path_text(path: &Path, model: &'static str) -> Result<String, SpeechError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or(SpeechError::Load(model))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::Instant;

    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

    use super::*;
    use crate::audio::resample::resample_clip;
    use crate::engine::local::{RECOGNIZER_DIR, RECOGNIZER_FILES};
    use crate::engine::{Stt, Tts, Vad};

    const THREADS: u8 = 2;
    const CHUNK_SAMPLES: usize = 1_600;
    const PRE_ROLL_CHUNKS: usize = 10;
    const SENTENCE: &str = "Hello there, this is a test of the voice proxy.";

    fn models_dir() -> PathBuf {
        let local = std::env::var_os("LOCALAPPDATA").expect("LOCALAPPDATA should be set");
        PathBuf::from(local)
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

    fn save_wav(path: &Path, samples: &[f32], sample_rate: u32) {
        let data: Vec<u8> = samples
            .iter()
            .flat_map(|sample| ((sample.clamp(-1.0, 1.0) * 32_767.0) as i16).to_le_bytes())
            .collect();
        let mut wav = Vec::new();
        wav.extend(b"RIFF");
        wav.extend((36 + data.len() as u32).to_le_bytes());
        wav.extend(b"WAVEfmt ");
        wav.extend(16_u32.to_le_bytes());
        wav.extend([1_u16, 1].iter().flat_map(|field| field.to_le_bytes()));
        wav.extend(sample_rate.to_le_bytes());
        wav.extend((sample_rate * 2).to_le_bytes());
        wav.extend([2_u16, 16].iter().flat_map(|field| field.to_le_bytes()));
        wav.extend(b"data");
        wav.extend((data.len() as u32).to_le_bytes());
        wav.extend(data);
        fs::write(path, wav).unwrap();
    }

    /// Speech padded with silence, at 16 kHz.
    fn microphone_audio(speech: &[f32], sample_rate: u32) -> Vec<f32> {
        let mut audio = vec![0.0; 16_000];
        audio.extend(resample_clip(speech, sample_rate, 16_000).unwrap());
        audio.extend(vec![0.0; 32_000]);
        audio
    }

    /// Recognizes the detected speech, including a little audio before it.
    fn recognize(vad: &mut SileroVad, stt: &mut ZipformerStt, audio: &[f32]) -> String {
        let mut recent: Vec<&[f32]> = Vec::new();
        let (mut was_speaking, mut text) = (false, String::new());
        for chunk in audio.chunks(CHUNK_SAMPLES) {
            let speaking = vad.accept(chunk);
            if speaking && !was_speaking {
                for earlier in &recent {
                    stt.accept(earlier);
                }
            }
            if speaking {
                stt.accept(chunk);
            } else if was_speaking {
                text = stt.finish();
            }
            was_speaking = speaking;
            recent.push(chunk);
            if recent.len() > PRE_ROLL_CHUNKS {
                recent.remove(0);
            }
        }
        text
    }

    #[test]
    #[ignore = "needs the models downloaded to the models directory"]
    fn spoken_text_is_recognized_again() {
        let models = models_dir();
        let lexicon = models.join("lexicon-en-us.txt");
        let mut vad = timed("load voice detection", || {
            SileroVad::load(&models.join("silero_vad.onnx")).unwrap()
        });
        let mut stt = timed("load recognition", || {
            ZipformerStt::load(&models.join(RECOGNIZER_DIR), RECOGNIZER_FILES, THREADS).unwrap()
        });
        let mut kokoro = timed("load Kokoro", || {
            SherpaTts::kokoro(&models.join("kokoro-int8-en-v0_19"), &lexicon, THREADS).unwrap()
        });
        let mut kitten = timed("load Kitten", || {
            SherpaTts::kitten(&models.join("kitten-nano-en-v0_8-int8"), &lexicon, THREADS).unwrap()
        });

        let samples_dir = models.join("samples");
        fs::create_dir_all(&samples_dir).unwrap();
        let voices: [(&str, &mut dyn Tts); 2] = [("kokoro", &mut kokoro), ("kitten", &mut kitten)];
        for (name, tts) in voices {
            let speech = timed(&format!("{name} synthesizes"), || {
                tts.synthesize(SENTENCE, 1.0).unwrap()
            });
            let seconds = speech.len() as f64 / f64::from(tts.sample_rate());
            println!("{name} speech: {seconds:.2} s at {} Hz", tts.sample_rate());
            timed(&format!("{name} synthesizes a short phrase"), || {
                tts.synthesize("Okay, sounds good.", 1.0).unwrap()
            });
            save_wav(
                &samples_dir.join(format!("{name}.wav")),
                &speech,
                tts.sample_rate(),
            );

            vad.reset();
            let audio = microphone_audio(&speech, tts.sample_rate());
            let text = timed(&format!("{name} speech recognized"), || {
                recognize(&mut vad, &mut stt, &audio)
            });
            println!("{name} recognized: {text:?}");
            let text = text.to_lowercase();
            assert!(text.contains("test") && text.contains("voice"), "{text}");
        }

        let silence = vec![0.0; 16_000 * 10];
        timed("10 s of silence through voice detection", || {
            for chunk in silence.chunks(CHUNK_SAMPLES) {
                vad.accept(chunk);
            }
        });
    }
}
