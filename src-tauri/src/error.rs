//! Errors shaped for display to a non technical user.

use serde::Serialize;
use specta::Type;

use crate::config::ConfigError;
use crate::engine::EngineError;

/// Machine readable identifier of a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    ConfigSaveFailed,
    EngineOff,
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
        }
    }
}
