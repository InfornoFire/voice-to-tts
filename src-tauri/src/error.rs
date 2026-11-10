//! Errors shaped for display to a non technical user.

use serde::Serialize;
use specta::Type;

use crate::audio::devices::AudioError;
use crate::config::ConfigError;
use crate::download::DownloadError;
use crate::engine::{EngineError, SpeechError};
use crate::hotkey::HotkeyError;
use crate::models::ModelsError;
use crate::subtitles::SubtitlesError;

/// Machine readable identifier of a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    ConfigSaveFailed,
    EngineOff,
    CableMissing,
    MicUnavailable,
    AudioUnavailable,
    ModelsUnavailable,
    ModelsDownloadFailed,
    SpeechFailed,
    HotkeyUnavailable,
    SubtitlesUnavailable,
}

/// The single recovery button the UI offers next to an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ErrorAction {
    Repair,
    PickMic,
    Retry,
}

/// A failure the user can see, with plain language text and at most one action.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct UserFacingError {
    pub code: ErrorCode,
    pub message: String,
    pub action: Option<ErrorAction>,
}

impl From<ConfigError> for UserFacingError {
    fn from(error: ConfigError) -> Self {
        match error {
            ConfigError::Write { .. } | ConfigError::Serialize(_) => Self {
                code: ErrorCode::ConfigSaveFailed,
                message: "Your settings could not be saved. Please try again.".to_owned(),
                action: Some(ErrorAction::Retry),
            },
        }
    }
}

impl From<EngineError> for UserFacingError {
    fn from(error: EngineError) -> Self {
        match error {
            EngineError::NotRunning => Self {
                code: ErrorCode::EngineOff,
                message: "Turn the switch on first, then try again.".to_owned(),
                action: None,
            },
            EngineError::Audio(error) => error.into(),
            EngineError::Speech(error) => error.into(),
        }
    }
}

impl From<SpeechError> for UserFacingError {
    fn from(error: SpeechError) -> Self {
        match error {
            SpeechError::Load(_) => Self {
                code: ErrorCode::ModelsUnavailable,
                message: "The speech models could not be loaded. Repairing them may help."
                    .to_owned(),
                action: Some(ErrorAction::Repair),
            },
            SpeechError::Synthesis => Self {
                code: ErrorCode::SpeechFailed,
                message: "That could not be spoken. Please try different text.".to_owned(),
                action: None,
            },
        }
    }
}

impl From<ModelsError> for UserFacingError {
    fn from(error: ModelsError) -> Self {
        let message = match error {
            ModelsError::Download(DownloadError::Download { .. }) => {
                "The speech models could not be downloaded. Check your internet connection and try again."
            }
            ModelsError::Download(DownloadError::Integrity { .. }) => {
                "A downloaded file was damaged, so it was not kept. Please try again."
            }
            ModelsError::Download(DownloadError::Write { .. }) => {
                "The speech models could not be saved. Check that your disk has free space and try again."
            }
            ModelsError::Download(DownloadError::Cancelled) => "The download was cancelled.",
            ModelsError::Interrupted => "The download stopped unexpectedly. Please try again.",
        };
        Self {
            code: ErrorCode::ModelsDownloadFailed,
            message: message.to_owned(),
            action: Some(ErrorAction::Retry),
        }
    }
}

impl From<HotkeyError> for UserFacingError {
    fn from(error: HotkeyError) -> Self {
        match error {
            HotkeyError::Unavailable(_) => Self {
                code: ErrorCode::HotkeyUnavailable,
                message: "That shortcut can't be used. Another app may already have it, so try a different one.".to_owned(),
                action: None,
            },
        }
    }
}

impl From<SubtitlesError> for UserFacingError {
    fn from(error: SubtitlesError) -> Self {
        match error {
            SubtitlesError::Unavailable(_) => Self {
                code: ErrorCode::SubtitlesUnavailable,
                message: "That port can't be used for the subtitles. Another app may already be using it, so try a different one.".to_owned(),
                action: None,
            },
        }
    }
}

impl From<AudioError> for UserFacingError {
    fn from(error: AudioError) -> Self {
        match error {
            AudioError::CableMissing => Self {
                code: ErrorCode::CableMissing,
                message: "The virtual microphone was removed. Turn the switch on again to use your audio output instead.".to_owned(),
                action: Some(ErrorAction::Retry),
            },
            AudioError::OpenCable(_) => Self {
                code: ErrorCode::CableMissing,
                message: "The virtual microphone could not be opened. If you just installed it, restart your computer first.".to_owned(),
                action: Some(ErrorAction::Retry),
            },
            AudioError::MicUnavailable | AudioError::OpenMic(_) => Self {
                code: ErrorCode::MicUnavailable,
                message: "Your microphone could not be used. Check that it is plugged in, or pick another one.".to_owned(),
                action: Some(ErrorAction::PickMic),
            },
            AudioError::OutputMissing | AudioError::OpenOutput(_) => Self {
                code: ErrorCode::AudioUnavailable,
                message: "No audio output could be used to play the voice. Check that your audio output is connected, then try again.".to_owned(),
                action: Some(ErrorAction::Retry),
            },
            AudioError::Host(_) | AudioError::Resample(_) => Self {
                code: ErrorCode::AudioUnavailable,
                message: "Your audio devices could not be read. Please try again.".to_owned(),
                action: Some(ErrorAction::Retry),
            },
        }
    }
}
