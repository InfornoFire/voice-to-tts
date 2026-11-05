import Setting from "./Setting";

import "./RangeField.css";

interface RangeFieldProps {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  format: (value: number) => string;
  hint?: string;
  onChange: (value: number) => void;
}

function RangeField({ label, value, min, max, step, format, hint, onChange }: RangeFieldProps) {
  return (
    <Setting label={label} value={format(value)} hint={hint}>
      <input
        class="range-field"
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onInput={(event) => onChange(event.currentTarget.valueAsNumber)}
      />
    </Setting>
  );
}

export default RangeField;
