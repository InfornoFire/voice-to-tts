import { useState } from "preact/hooks";

import Setting from "./Setting";

interface NumberFieldProps {
  label: string;
  value: number;
  min: number;
  max: number;
  hint?: string;
  /** Called with a valid whole number when the user leaves the field, never while typing. */
  onChange: (value: number) => void;
}

function NumberField({ label, value, min, max, hint, onChange }: NumberFieldProps) {
  const [draft, setDraft] = useState<string | null>(null);

  const commit = () => {
    const entered = Number(draft);
    if (draft !== null && Number.isInteger(entered) && entered >= min && entered <= max) {
      onChange(entered);
    }
    setDraft(null);
  };

  return (
    <Setting label={label} hint={hint}>
      <input
        class="field"
        type="number"
        inputMode="numeric"
        min={min}
        max={max}
        step={1}
        value={draft ?? String(value)}
        onInput={(event) => setDraft(event.currentTarget.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.currentTarget.blur();
          }
        }}
      />
    </Setting>
  );
}

export default NumberField;
