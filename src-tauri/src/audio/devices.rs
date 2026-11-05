use cpal::traits::{DeviceTrait, HostTrait};
use serde::Serialize;
use specta::Type;

/// Adapter name of both VB-CABLE endpoints. It stays in the endpoint name when a user renames it.
const CABLE_ADAPTER_NAME: &str = "vb-audio virtual cable";

/// Failure to find or read the audio devices.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("the virtual microphone is not installed")]
    CableMissing,
    #[error("could not read the audio devices")]
    Host(#[from] cpal::Error),
}

/// A microphone the user can pick.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct InputDevice {
    /// Stable identifier to store in the config.
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

/// A device as listed by the audio host.
struct Endpoint {
    id: String,
    name: String,
}

impl Endpoint {
    fn of(device: &impl DeviceTrait) -> Option<Self> {
        Some(Self {
            id: device.id().ok()?.to_string(),
            name: device.description().ok()?.name().to_owned(),
        })
    }
}

/// Lists the microphones that can be captured, without the virtual microphone's own output.
pub fn list_input_devices() -> Result<Vec<InputDevice>, AudioError> {
    let host = cpal::default_host();
    let default_id = host
        .default_input_device()
        .and_then(|device| Some(device.id().ok()?.to_string()));
    let endpoints = host
        .input_devices()?
        .filter_map(|device| Endpoint::of(&device));
    Ok(microphones(endpoints, default_id.as_deref()))
}

/// Succeeds when the virtual microphone is installed.
pub fn check_cable() -> Result<(), AudioError> {
    let host = cpal::default_host();
    let installed = host
        .output_devices()?
        .filter_map(|device| Endpoint::of(&device))
        .any(|endpoint| is_cable(&endpoint.name));
    if installed {
        Ok(())
    } else {
        Err(AudioError::CableMissing)
    }
}

fn is_cable(name: &str) -> bool {
    name.to_lowercase().contains(CABLE_ADAPTER_NAME)
}

fn microphones(
    endpoints: impl IntoIterator<Item = Endpoint>,
    default_id: Option<&str>,
) -> Vec<InputDevice> {
    endpoints
        .into_iter()
        .filter(|endpoint| !is_cable(&endpoint.name))
        .map(|endpoint| InputDevice {
            is_default: default_id == Some(endpoint.id.as_str()),
            id: endpoint.id,
            name: endpoint.name,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(id: &str, name: &str) -> Endpoint {
        Endpoint {
            id: id.to_owned(),
            name: name.to_owned(),
        }
    }

    #[test]
    fn microphones_exclude_the_cable_and_mark_the_default() {
        let endpoints = [
            endpoint("first", "Microphone A (Adapter A)"),
            endpoint("cable", &format!("Renamed ({CABLE_ADAPTER_NAME})")),
            endpoint("uppercase", &CABLE_ADAPTER_NAME.to_uppercase()),
            endpoint("second", "Microphone B (Adapter B)"),
        ];

        let devices = microphones(endpoints, Some("second"));

        let listed: Vec<_> = devices
            .iter()
            .map(|device| (device.id.as_str(), device.is_default))
            .collect();
        assert_eq!(listed, [("first", false), ("second", true)]);
    }
}
