import { signal } from "@preact/signals";

import {
  type InputDevice,
  type UserFacingError,
  commands,
  toUserFacingError,
  unwrap,
} from "../ipc";
import { logFailure } from "../lib/log";

export const inputDevices = signal<InputDevice[]>([]);

/** Whether the virtual microphone is installed, or null until it has been checked. */
export const isCableInstalled = signal<boolean | null>(null);

/** Why the microphone list could not be read. */
export const deviceError = signal<UserFacingError | null>(null);

export async function refreshInputDevices() {
  try {
    inputDevices.value = await unwrap(commands.listInputDevices());
    deviceError.value = null;
  } catch (error) {
    deviceError.value = toUserFacingError(error);
  }
}

async function checkCable() {
  isCableInstalled.value = await commands.isCableInstalled();
}

const recheckCable = () => {
  checkCable().catch(logFailure("Failed to check for the virtual microphone."));
};

/** Checks for the virtual microphone now and whenever the window regains focus. */
export function syncDevices() {
  recheckCable();
  window.addEventListener("focus", recheckCable);
}
