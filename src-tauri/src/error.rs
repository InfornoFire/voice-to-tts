//! Errors shaped for display to a non technical user.

use serde::Serialize;
use specta::Type;

use crate::audio::devices::AudioError;
use crate::config::ConfigError;
use crate::engine::EngineError;
use crate::hotkey::HotkeyError;

/// Machine readable identifier of a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    ConfigSaveFailed,
    EngineOff,
    CableMissing,
    MicUnavailable,
    AudioUnavailable,
    HotkeyUnavailable,
}

/// The single recovery button the UI offers next to an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ErrorAction {
    Repair,
    Restart,
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

impl From<AudioError> for UserFacingError {
    fn from(error: AudioError) -> Self {
        match error {
            AudioError::CableMissing => Self {
                code: ErrorCode::CableMissing,
                message: "The virtual microphone is not installed.".to_owned(),
                action: Some(ErrorAction::Repair),
            },
            AudioError::OpenCable(_) => Self {
                code: ErrorCode::CableMissing,
                message: "The virtual microphone could not be opened.".to_owned(),
                action: Some(ErrorAction::Repair),
            },
            AudioError::MicUnavailable | AudioError::OpenMic(_) => Self {
                code: ErrorCode::MicUnavailable,
                message: "Your microphone could not be used. Check that it is plugged in, or pick another one.".to_owned(),
                action: Some(ErrorAction::PickMic),
            },
            AudioError::Host(_) | AudioError::Resample(_) => Self {
                code: ErrorCode::AudioUnavailable,
                message: "Your audio devices could not be read. Please try again.".to_owned(),
                action: Some(ErrorAction::Retry),
            },
        }
    }
}
