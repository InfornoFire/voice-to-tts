import { computed, signal } from "@preact/signals";

import {
  type EngineState,
  type ErrorAction,
  type ErrorCode,
  type UserFacingError,
  commands,
  events,
  toUserFacingError,
  unwrap,
} from "../ipc";
import { VB_CABLE_URL, openLink } from "../lib/links";
import { cableError, checkCable } from "./devices";
import { downloadModels } from "./models";
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

/** Whether the switch can be used. A running engine can always be switched off. */
export const canToggleEngine = computed(() => isEngineOn.value || cableError.value === null);

/** The failure to show the user, from the virtual microphone, the last command or the engine. */
export const activeError = computed(() => {
  const state = engineState.value;
  return cableError.value ?? commandError.value ?? (state.status === "error" ? state.error : null);
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

const TEST_SENTENCE = "Hello, this is a test of voice-to-tts.";

/** Speaks a short sentence, so the input level of a chat app can be watched. */
export function playTestSentence(): Promise<boolean> {
  return speakText(TEST_SENTENCE);
}

/** Speaks `text`, resolving with whether the engine accepted it. */
export async function speakText(text: string): Promise<boolean> {
  commandError.value = null;
  try {
    await unwrap(commands.speakText(text));
    return true;
  } catch (error) {
    commandError.value = toUserFacingError(error);
    return false;
  }
}

/** Does what the button on an error offers. Restart has nothing to run yet. */
export function runErrorAction(action: ErrorAction, code: ErrorCode) {
  if (action === "retry") {
    void retry();
  } else if (action === "pick_mic") {
    navigate("settings");
  } else if (action === "install_cable") {
    openLink(VB_CABLE_URL);
  } else if (action === "repair" && code === "models_unavailable") {
    void repairModels();
  }
}

/** Downloads the missing or damaged models, then switches the engine on again if they stopped it. */
export async function repairModels() {
  const state = engineState.peek();
  const stoppedByModels = state.status === "error" && state.error.code === "models_unavailable";
  if ((await downloadModels()) && stoppedByModels) {
    await retry();
  }
}

async function retry() {
  await checkCable();
  if (cableError.value === null) {
    await setEnabled(true);
  }
}

async function setEnabled(enabled: boolean) {
  commandError.value = null;
  try {
    await unwrap(commands.setEnabled(enabled));
  } catch (error) {
    commandError.value = toUserFacingError(error);
  }
}
