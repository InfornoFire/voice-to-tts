import { useState } from "preact/hooks";

import "./SpeakBox.css";

interface SpeakBoxProps {
  disabled: boolean;
  /** Resolves with whether the text was accepted, so only then is the box cleared. */
  onSpeak: (text: string) => Promise<boolean>;
}

function SpeakBox({ disabled, onSpeak }: SpeakBoxProps) {
  const [text, setText] = useState("");
  const message = text.trim();

  const submit = async () => {
    if (await onSpeak(message)) {
      setText("");
    }
  };

  return (
    <form
      class="speak-box"
      onSubmit={(event) => {
        event.preventDefault();
        void submit();
      }}
    >
      <input
        class="field speak-box-input"
        type="text"
        value={text}
        disabled={disabled}
        placeholder={disabled ? "Turn the switch on to type a message" : "Type something to say"}
        aria-label="Message to speak"
        onInput={(event) => setText(event.currentTarget.value)}
      />
      <button type="submit" class="button" disabled={disabled || message === ""}>
        Speak
      </button>
    </form>
  );
}

export default SpeakBox;
