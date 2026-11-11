import { useEffect, useRef, useState } from "preact/hooks";

import type { CloseChoice } from "../ipc";
import { logFailure } from "../lib/log";
import { answerCloseRequest, dismissClosePrompt } from "../state/close";
import CheckboxField from "./CheckboxField";

import "./ClosePrompt.css";

/** Asks whether closing the window should keep the app running in the tray. */
function ClosePrompt() {
  const dialog = useRef<HTMLDialogElement>(null);
  const [remember, setRemember] = useState(false);

  useEffect(() => {
    dialog.current?.showModal();
  }, []);

  const answer = (choice: CloseChoice) => {
    answerCloseRequest(choice, remember).catch(logFailure("Failed to close the window."));
  };

  return (
    <dialog
      ref={dialog}
      class="close-prompt"
      aria-labelledby="close-prompt-title"
      onCancel={dismissClosePrompt}
    >
      <h2 id="close-prompt-title" class="close-prompt-title">
        Close voice-to-tts?
      </h2>
      <p class="close-prompt-message">
        Minimize to the tray to keep the shortcut and the voice working in the background. Quit to
        stop the app completely.
      </p>
      <CheckboxField
        label="Remember my choice"
        checked={remember}
        hint="You can change this in the settings."
        onChange={setRemember}
      />
      <div class="close-prompt-actions">
        <button type="button" class="button" onClick={dismissClosePrompt}>
          Cancel
        </button>
        <button type="button" class="button" onClick={() => answer("quit")}>
          Quit
        </button>
        <button
          type="button"
          class="button button-primary"
          autofocus
          onClick={() => answer("tray")}
        >
          Minimize to tray
        </button>
      </div>
    </dialog>
  );
}

export default ClosePrompt;
