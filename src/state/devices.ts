import { signal } from "@preact/signals";

import {
  type InputDevice,
  type UserFacingError,
  commands,
  toUserFacingError,
  unwrap,
} from "../ipc";
import { config, saveConfig } from "./config";

export const inputDevices = signal<InputDevice[]>([]);

/** Why the virtual microphone cannot be used, or null when it can. */
export const cableError = signal<UserFacingError | null>(null);

/** Why the microphone list could not be read or the choice saved. */
export const deviceError = signal<UserFacingError | null>(null);

export async function checkCable() {
  try {
    await unwrap(commands.checkCable());
    cableError.value = null;
  } catch (error) {
    cableError.value = toUserFacingError(error);
  }
}

export async function refreshInputDevices() {
  try {
    inputDevices.value = await unwrap(commands.listInputDevices());
    deviceError.value = null;
  } catch (error) {
    deviceError.value = toUserFacingError(error);
  }
}

/** Saves the microphone to capture from, where `null` follows the system default. */
export async function selectInputDevice(deviceId: string | null) {
  const current = config.peek();
  if (!current) {
    return;
  }
  try {
    await saveConfig({ ...current, input_device: deviceId });
    deviceError.value = null;
  } catch (error) {
    deviceError.value = toUserFacingError(error);
  }
}

const recheckCable = () => {
  void checkCable();
};

/** Checks the virtual microphone now and whenever the window regains focus. */
export function syncDevices() {
  recheckCable();
  window.addEventListener("focus", recheckCable);
}
