import { computed, signal } from "@preact/signals";

import {
  CommandError,
  type EngineState,
  type ErrorAction,
  type UserFacingError,
  commands,
  events,
  unwrap,
} from "../ipc";
import { navigate } from "./view";

export const engineState = signal<EngineState>({ status: "off" });

const commandError = signal<UserFacingError | null>(null);

/** Whether the engine is starting or up. */
export const isEngineOn = computed(() => {
  const { status } = engineState.value;
  return status === "loading" || status === "listening" || status === "speaking";
});

/** Whether the engine is up and can take text to speak. */
export const isEngineRunning = computed(() => {
  const { status } = engineState.value;
  return status === "listening" || status === "speaking";
});

/** The failure to show the user, from the engine itself or from the last command. */
export const activeError = computed(() => {
  const state = engineState.value;
  return commandError.value ?? (state.status === "error" ? state.error : null);
});

/** Keeps `engineState` in step with the backend. */
export async function syncEngineState() {
  let receivedEvent = false;
  await events.stateChanged.listen((event) => {
    receivedEvent = true;
    engineState.value = event.payload;
  });

  // Listening comes first so no change is missed, and the snapshot never overrides an event.
  const snapshot = await commands.getEngineState();
  if (!receivedEvent) {
    engineState.value = snapshot;
  }
}

export function toggleEngine() {
  void setEnabled(!isEngineOn.value);
}

/** Speaks `text`, resolving with whether the engine accepted it. */
export async function speakText(text: string): Promise<boolean> {
  commandError.value = null;
  try {
    await unwrap(commands.speakText(text));
    return true;
  } catch (error) {
    reportCommandFailure(error);
    return false;
  }
}

/** Does what the button on an error offers. Repair and restart have nothing to run yet. */
export function runErrorAction(action: ErrorAction) {
  if (action === "retry") {
    void setEnabled(true);
  } else if (action === "pick_mic") {
    navigate("settings");
  }
}

async function setEnabled(enabled: boolean) {
  commandError.value = null;
  try {
    await unwrap(commands.setEnabled(enabled));
  } catch (error) {
    reportCommandFailure(error);
  }
}

function reportCommandFailure(error: unknown) {
  if (error instanceof CommandError) {
    commandError.value = error.detail;
  } else {
    console.error("A command failed unexpectedly.", error);
  }
}
