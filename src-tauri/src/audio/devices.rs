use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, DeviceId};
use serde::Serialize;
use specta::Type;

/// Adapter name shared by both VB-CABLE endpoints.
const CABLE_ADAPTER_NAME: &str = "vb-audio virtual cable";

/// Failure to find, read or open the audio devices.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("the virtual microphone is not installed")]
    CableMissing,
    #[error("no usable microphone was found")]
    MicUnavailable,
    #[error("could not read the audio devices")]
    Host(#[from] cpal::Error),
    #[error("could not open the microphone")]
    OpenMic(#[source] cpal::Error),
    #[error("could not open the virtual microphone")]
    OpenCable(#[source] cpal::Error),
    #[error("could not set up audio conversion")]
    Resample(#[from] rubato::ResamplerConstructionError),
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
    find_cable_output().map(drop)
}

/// Finds the input of the virtual microphone.
pub fn find_cable_output() -> Result<Device, AudioError> {
    cpal::default_host()
        .output_devices()?
        .find(|device| device_matches(device, is_cable))
        .ok_or(AudioError::CableMissing)
}

/// Finds the microphone to capture from, where `None` follows the system default.
///
/// The virtual microphone is never returned.
pub fn find_input_device(device_id: Option<&str>) -> Result<Device, AudioError> {
    let host = cpal::default_host();
    let is_microphone =
        |device: &Device| device.supports_input() && device_matches(device, |name| !is_cable(name));

    let device = match device_id {
        Some(device_id) => {
            let device_id: DeviceId = device_id.parse().map_err(|_| AudioError::MicUnavailable)?;
            host.device_by_id(&device_id)
        }
        None => host
            .default_input_device()
            .and_then(|device| host.device_by_id(&device.id().ok()?))
            .filter(is_microphone)
            .or_else(|| host.input_devices().ok()?.find(is_microphone)),
    };
    device
        .filter(is_microphone)
        .ok_or(AudioError::MicUnavailable)
}

/// Whether the device has a readable name that satisfies `predicate`.
fn device_matches(device: &Device, predicate: impl Fn(&str) -> bool) -> bool {
    device
        .description()
        .is_ok_and(|description| predicate(description.name()))
}

pub(crate) fn is_cable(name: &str) -> bool {
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
