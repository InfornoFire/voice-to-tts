import { signal } from "@preact/signals";

import { type ModelsState, type Voice, commands, events, toUserFacingError, unwrap } from "../ipc";

/** What the speech models are doing, or null until the backend has said. */
export const modelsState = signal<ModelsState | null>(null);

/** The voices to pick from, the first being the default. */
export const voices = signal<Voice[]>([]);

/** Keeps `modelsState` in step with the backend and loads the voices. */
export async function syncModels() {
  let receivedEvent = false;
  await events.modelsStateChanged.listen((event) => {
    receivedEvent = true;
    modelsState.value = event.payload;
  });

  // Listening comes first so no change is missed, and the snapshot never overrides an event.
  const snapshot = await commands.getModelsState();
  if (!receivedEvent) {
    modelsState.value = snapshot;
  }
  voices.value = await commands.listVoices();
}

/** Downloads the missing or damaged models, resolving with whether they are ready. */
export async function downloadModels(): Promise<boolean> {
  try {
    const state = await unwrap(commands.downloadModels());
    return state.status === "ready";
  } catch (error) {
    const detail = toUserFacingError(error);
    if (detail) {
      modelsState.value = { status: "failed", error: detail };
    }
    return false;
  }
}

export function cancelDownload() {
  commands
    .cancelModelsDownload()
    .catch((error: unknown) => console.error("Failed to cancel the download.", error));
}
