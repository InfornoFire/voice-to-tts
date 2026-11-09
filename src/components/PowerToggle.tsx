import "./PowerToggle.css";

interface PowerToggleProps {
  checked: boolean;
  onToggle: () => void;
}

function PowerToggle({ checked, onToggle }: PowerToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label="Voice"
      class="power-toggle"
      onClick={onToggle}
    >
      <span class="power-toggle-knob" />
    </button>
  );
}

export default PowerToggle;
