import "./PowerToggle.css";

interface PowerToggleProps {
  checked: boolean;
  disabled: boolean;
  onToggle: () => void;
}

function PowerToggle({ checked, disabled, onToggle }: PowerToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label="Voice"
      class="power-toggle"
      disabled={disabled}
      onClick={onToggle}
    >
      <span class="power-toggle-knob" />
    </button>
  );
}

export default PowerToggle;
