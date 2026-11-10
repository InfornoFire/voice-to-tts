import Setting from "./Setting";

interface ReadOnlyFieldProps {
  label: string;
  value: string;
  hint?: string;
}

function ReadOnlyField({ label, value, hint }: ReadOnlyFieldProps) {
  return (
    <Setting label={label} hint={hint}>
      <input
        class="field"
        type="text"
        readOnly
        value={value}
        onFocus={(event) => event.currentTarget.select()}
      />
    </Setting>
  );
}

export default ReadOnlyField;
