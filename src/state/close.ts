import { signal } from "@preact/signals";

import { type CloseChoice, commands, events } from "../ipc";
import { config } from "./config";

/** Whether the user is being asked what closing the window should do. */
export const isClosePromptOpen = signal(false);

/** Opens the prompt whenever the backend needs to know what closing the window should do. */
export function syncCloseRequests() {
  return events.closeRequested.listen(() => {
    isClosePromptOpen.value = true;
  });
}

/** Leaves the window open without answering. */
export function dismissClosePrompt() {
  isClosePromptOpen.value = false;
}

/** Closes the window as chosen, and makes that the answer from now on when `remember` is set. */
export function answerCloseRequest(choice: CloseChoice, remember: boolean) {
  isClosePromptOpen.value = false;
  const current = config.peek();
  if (remember && current) {
    // The backend stores this itself, so it is shown without going through a save.
    config.value = { ...current, close_action: choice };
  }
  return commands.answerCloseRequest(choice, remember);
}
