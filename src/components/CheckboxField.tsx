import "./CheckboxField.css";

interface CheckboxFieldProps {
  label: string;
  checked: boolean;
  disabled?: boolean;
  hint?: string;
  onChange: (checked: boolean) => void;
}

function CheckboxField({ label, checked, disabled, hint, onChange }: CheckboxFieldProps) {
  return (
    <div class="checkbox-field">
      <label class="checkbox-field-label">
        <input
          type="checkbox"
          checked={checked}
          disabled={disabled}
          onChange={(event) => onChange(event.currentTarget.checked)}
        />
        <span>{label}</span>
      </label>
      {hint && <p class="hint">{hint}</p>}
    </div>
  );
}

export default CheckboxField;
