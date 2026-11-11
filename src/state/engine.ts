import { computed, signal } from "@preact/signals";

import {
  type EngineState,
  type UserFacingError,
  commands,
  events,
  toUserFacingError,
  unwrap,
} from "../ipc";
import { logFailure } from "../lib/log";
import { downloadModels } from "./models";
import { syncWithBackend } from "./sync";
import { navigate } from "./view";

export const engineState = signal<EngineState>({ status: "off" });

const commandError = signal<UserFacingError | null>(null);

/** Whether the engine is starting or up. */
export const isEngineOn = computed(() => {
  const { status } = engineState.value;
  return status !== "off" && status !== "error";
});

/** Whether the engine is up and can take text to speak. */
export const isEngineRunning = computed(() => {
  const { status } = engineState.value;
  return status === "listening" || status === "speaking";
});

/** The failure to show the user, from the last command or the engine. */
export const activeError = computed(() => {
  const state = engineState.value;
  return commandError.value ?? (state.status === "error" ? state.error : null);
});

/** Keeps `engineState` in step with the backend. */
export function syncEngineState() {
  return syncWithBackend(
    (handler) => events.stateChanged.listen(handler),
    () => commands.getEngineState(),
    (state) => {
      engineState.value = state;
    },
  );
}

export function toggleEngine() {
  void setEnabled(!isEngineOn.value);
}

/** Stops the voice and pauses listening for a moment. */
export function cancelSpeech() {
  commands.cancelSpeech().catch(logFailure("Failed to cancel the speech."));
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

/** Does what the button on `error` offers. */
export function runErrorAction({ action, code }: UserFacingError) {
  if (action === "retry") {
    void setEnabled(true);
  } else if (action === "pick_mic") {
    navigate("settings");
  } else if (action === "repair" && code === "models_unavailable") {
    void repairModels();
  }
}

/** Downloads the missing or damaged models, then switches the engine on again if they stopped it. */
export async function repairModels() {
  const state = engineState.peek();
  const stoppedByModels = state.status === "error" && state.error.code === "models_unavailable";
  if ((await downloadModels()) && stoppedByModels) {
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
