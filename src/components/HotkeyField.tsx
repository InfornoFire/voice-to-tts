import { useEffect, useState } from "preact/hooks";

import { formatShortcut, readShortcut } from "../lib/shortcut";
import Setting from "./Setting";

import "./HotkeyField.css";

const DEFAULT_HINT = "Select it, then press the keys you want. Backspace turns the shortcut off.";
const REJECTED_HINT = "Hold Ctrl, Alt or the Windows key while you press another key.";

interface HotkeyFieldProps {
  label: string;
  /** The current shortcut, where an empty value means none. */
  value: string;
  onChange: (shortcut: string) => void;
  /** Called with `true` while a shortcut is being recorded and `false` once it stops. */
  onRecordingChange: (isRecording: boolean) => void;
}

function HotkeyField({ label, value, onChange, onRecordingChange }: HotkeyFieldProps) {
  const [isRecording, setIsRecording] = useState(false);
  const [wasRejected, setWasRejected] = useState(false);

  useEffect(() => {
    if (!isRecording) {
      return undefined;
    }
    onRecordingChange(true);
    return () => onRecordingChange(false);
  }, [isRecording, onRecordingChange]);

  const stopRecording = () => {
    setIsRecording(false);
    setWasRejected(false);
  };

  const handleKeyDown = (event: KeyboardEvent) => {
    if (!isRecording) {
      return;
    }
    const reading = readShortcut(event);
    if (reading.kind === "rejected" && event.code === "Tab") {
      return;
    }
    event.preventDefault();
    if (event.repeat) {
      return;
    }

    switch (reading.kind) {
      case "accepted":
        onChange(reading.shortcut);
        stopRecording();
        break;
      case "clear":
        onChange("");
        stopRecording();
        break;
      case "cancel":
        stopRecording();
        break;
      case "rejected":
        setWasRejected(true);
        break;
      case "incomplete":
        break;
    }
  };

  return (
    <Setting label={label} hint={wasRejected ? REJECTED_HINT : DEFAULT_HINT}>
      <button
        type="button"
        class="field hotkey-field"
        data-recording={isRecording ? "true" : "false"}
        onClick={() => setIsRecording(true)}
        onBlur={stopRecording}
        onKeyDown={handleKeyDown}
      >
        {isRecording ? "Press the new shortcut…" : formatShortcut(value)}
      </button>
    </Setting>
  );
}

export default HotkeyField;
