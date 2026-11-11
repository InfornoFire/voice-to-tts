import Setting from "./Setting";

export interface SelectOption<Value extends string = string> {
  value: Value;
  label: string;
}

interface SelectFieldProps<Value extends string> {
  label: string;
  value: Value;
  options: SelectOption<Value>[];
  hint?: string;
  onChange: (value: Value) => void;
}

function SelectField<Value extends string>({
  label,
  value,
  options,
  hint,
  onChange,
}: SelectFieldProps<Value>) {
  const choose = (selected: string) => {
    const option = options.find((candidate) => candidate.value === selected);
    if (option) {
      onChange(option.value);
    }
  };

  return (
    <Setting label={label} hint={hint}>
      <select class="field" value={value} onChange={(event) => choose(event.currentTarget.value)}>
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
