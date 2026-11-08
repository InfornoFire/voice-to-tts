import { signal } from "@preact/signals";

import {
  type InputDevice,
  type UserFacingError,
  commands,
  toUserFacingError,
  unwrap,
} from "../ipc";

export const inputDevices = signal<InputDevice[]>([]);

/** Why the virtual microphone cannot be used, or null when it can or has not been checked yet. */
export const cableError = signal<UserFacingError | null>(null);

/** Whether the virtual microphone has been checked at least once. */
export const isCableChecked = signal(false);

/** Why the microphone list could not be read. */
export const deviceError = signal<UserFacingError | null>(null);

export async function checkCable() {
  try {
    await unwrap(commands.checkCable());
    cableError.value = null;
  } catch (error) {
    cableError.value = toUserFacingError(error);
  }
  isCableChecked.value = true;
}

export async function refreshInputDevices() {
  try {
    inputDevices.value = await unwrap(commands.listInputDevices());
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
