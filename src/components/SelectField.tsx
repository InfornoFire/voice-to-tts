import Setting from "./Setting";

export interface SelectOption {
  value: string;
  label: string;
}

interface SelectFieldProps {
  label: string;
  value: string;
  options: SelectOption[];
  hint?: string;
  onChange: (value: string) => void;
}

function SelectField({ label, value, options, hint, onChange }: SelectFieldProps) {
  return (
    <Setting label={label} hint={hint}>
      <select class="field" value={value} onChange={(event) => onChange(event.currentTarget.value)}>
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </Setting>
  );
}

export default SelectField;
