//! User settings and their persistence.

mod store;

use serde::{Deserialize, Serialize};
use specta::Type;

pub use store::{ConfigError, ConfigStore};

pub const MIN_THREADS: u8 = 1;
pub const MAX_THREADS: u8 = 8;
pub const MIN_TTS_SPEED: f32 = 0.5;
pub const MAX_TTS_SPEED: f32 = 2.0;
pub const MIN_SUBTITLES_PORT: u16 = 1024;

const DEFAULT_HOTKEY: &str = "Ctrl+Alt+V";
const DEFAULT_TTS_VOICE: &str = "default";
const DEFAULT_TTS_SPEED: f32 = 1.0;
const DEFAULT_THREADS: u8 = 2;
const DEFAULT_SUBTITLES_PORT: u16 = 7421;

/// Persisted user settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Config {
    /// Identifier of the microphone to capture from, or `None` for the system default.
    pub input_device: Option<String>,
    /// Global shortcut that toggles the engine. An empty value disables the shortcut.
    pub hotkey: String,
    /// Name of the text to speech voice.
    pub tts_voice: String,
    /// Playback speed multiplier between [`MIN_TTS_SPEED`] and [`MAX_TTS_SPEED`].
    pub tts_speed: f32,
    /// Whether the voice also plays through the audio output while it goes to the virtual
    /// microphone.
    pub hear_voice: bool,
    /// Whether the spoken text is served to an OBS browser source.
    pub subtitles_enabled: bool,
    /// Port of the subtitles page, which is then at `http://127.0.0.1:<port>`. At least
    /// [`MIN_SUBTITLES_PORT`].
    pub subtitles_port: u16,
    /// Maximum number of inference threads, between [`MIN_THREADS`] and [`MAX_THREADS`].
    pub threads: u8,
    /// Whether the main window stays hidden on launch.
    pub start_minimized: bool,
    /// Whether the user has been through the setup guide.
    pub setup_guide_done: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            input_device: None,
            hotkey: DEFAULT_HOTKEY.to_owned(),
            tts_voice: DEFAULT_TTS_VOICE.to_owned(),
            tts_speed: DEFAULT_TTS_SPEED,
            hear_voice: false,
            subtitles_enabled: false,
            subtitles_port: DEFAULT_SUBTITLES_PORT,
            threads: DEFAULT_THREADS,
            start_minimized: false,
            setup_guide_done: false,
        }
    }
}

impl Config {
    /// Returns the config with every value brought into its valid range.
    pub fn normalized(mut self) -> Self {
        self.input_device = self
            .input_device
            .filter(|device_id| !device_id.trim().is_empty());
        self.hotkey = self.hotkey.trim().to_owned();

        let voice = self.tts_voice.trim();
        self.tts_voice = if voice.is_empty() {
            DEFAULT_TTS_VOICE.to_owned()
        } else {
            voice.to_owned()
        };

        self.tts_speed = if self.tts_speed.is_finite() {
            self.tts_speed.clamp(MIN_TTS_SPEED, MAX_TTS_SPEED)
        } else {
            DEFAULT_TTS_SPEED
        };
        self.threads = self.threads.clamp(MIN_THREADS, MAX_THREADS);

        if self.subtitles_port < MIN_SUBTITLES_PORT {
            self.subtitles_port = DEFAULT_SUBTITLES_PORT;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_already_normalized() {
        assert_eq!(Config::default().normalized(), Config::default());
    }

    #[test]
    fn numbers_are_clamped_into_range() {
        let low = Config {
            threads: 0,
            tts_speed: 0.1,
            ..Config::default()
        }
        .normalized();
        let high = Config {
            threads: 200,
            tts_speed: 9.0,
            ..Config::default()
        }
        .normalized();

        assert_eq!((low.threads, low.tts_speed), (MIN_THREADS, MIN_TTS_SPEED));
        assert_eq!((high.threads, high.tts_speed), (MAX_THREADS, MAX_TTS_SPEED));
    }

    #[test]
    fn non_finite_tts_speed_resets_to_the_default() {
        for tts_speed in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let config = Config {
                tts_speed,
                ..Config::default()
            };

            assert_eq!(config.normalized().tts_speed, DEFAULT_TTS_SPEED);
        }
    }

    #[test]
    fn a_subtitles_port_below_the_minimum_resets_to_the_default() {
        let cases = [
            (0, DEFAULT_SUBTITLES_PORT),
            (MIN_SUBTITLES_PORT - 1, DEFAULT_SUBTITLES_PORT),
            (MIN_SUBTITLES_PORT, MIN_SUBTITLES_PORT),
            (u16::MAX, u16::MAX),
        ];

        for (subtitles_port, expected) in cases {
            let config = Config {
                subtitles_port,
                ..Config::default()
            };

            assert_eq!(config.normalized().subtitles_port, expected);
        }
    }

    #[test]
    fn text_values_are_trimmed_and_blanks_reset() {
        let blank = Config {
            input_device: Some(" ".to_owned()),
            hotkey: " ".to_owned(),
            tts_voice: " ".to_owned(),
            ..Config::default()
        }
        .normalized();
        let padded = Config {
            hotkey: " Ctrl+Shift+T ".to_owned(),
            tts_voice: " amy ".to_owned(),
            ..Config::default()
        }
        .normalized();

        assert_eq!(blank.input_device, None);
        assert_eq!(blank.hotkey, "");
        assert_eq!(blank.tts_voice, DEFAULT_TTS_VOICE);
        assert_eq!(padded.hotkey, "Ctrl+Shift+T");
        assert_eq!(padded.tts_voice, "amy");
    }
}
