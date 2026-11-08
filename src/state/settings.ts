import { signal } from "@preact/signals";

import { type Config, type UserFacingError, commands, toUserFacingError, unwrap } from "../ipc";
import { config } from "./config";
import { navigate } from "./view";

const SAVE_DELAY_MS = 400;

/** Limits of the settings, matching `config.rs`. */
export const TTS_SPEED = { min: 0.5, max: 2, step: 0.1, fallback: 1 };
export const THREAD_COUNTS = [1, 2, 3, 4, 5, 6, 7, 8];

/** Why the settings could not be saved. */
export const settingsError = signal<UserFacingError | null>(null);

let edits = 0;
let saveTimer: ReturnType<typeof setTimeout> | undefined;
let saving: Promise<void> = Promise.resolve();

/** Shows `changes` right away and saves them once the user stops adjusting. */
export function updateSettings(changes: Partial<Config>) {
  const current = config.peek();
  if (!current) {
    return;
  }
  config.value = { ...current, ...changes };
  edits += 1;
  clearTimeout(saveTimer);
  saveTimer = setTimeout(saveSettings, SAVE_DELAY_MS);
}

/** Remembers that the setup guide was seen, and goes to the main view. */
export function finishGuide() {
  updateSettings({ setup_guide_done: true });
  navigate("main");
}

/** Stops the global shortcut from switching the engine while a new one is being recorded. */
export function pauseHotkey(paused: boolean) {
  unwrap(commands.setHotkeyPaused(paused)).catch((error: unknown) =>
    console.error("Failed to pause the shortcut.", error),
  );
}

/** Saves the settings as shown now. */
export function saveSettings() {
  clearTimeout(saveTimer);
  const shown = config.peek();
  if (shown) {
    const editsAtSave = edits;
    saving = saving.then(() => persist(shown, editsAtSave));
  }
}

async function persist(shown: Config, editsAtSave: number) {
  try {
    const stored = await unwrap(commands.updateConfig(shown));
    settingsError.value = null;
    // The stored copy is normalized, but newer edits on screen must not be undone by it.
    if (edits === editsAtSave) {
      config.value = stored;
    }
  } catch (error) {
    settingsError.value = toUserFacingError(error);
  }
}
