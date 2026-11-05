import type { InputDevice } from "../ipc";

import "./MicPicker.css";

interface MicPickerProps {
  devices: InputDevice[];
  /** The stored device, or `null` for the system default. */
  selectedId: string | null;
  disabled: boolean;
  onSelect: (deviceId: string | null) => void;
}

function MicPicker({ devices, selectedId, disabled, onSelect }: MicPickerProps) {
  const isDisconnected = selectedId !== null && !devices.some((device) => device.id === selectedId);

  return (
    <div class="mic-picker">
      <label class="mic-picker-label">
        <span>Microphone</span>
        <select
          class="field"
          value={selectedId ?? ""}
          disabled={disabled}
          onChange={(event) => onSelect(event.currentTarget.value || null)}
        >
          <option value="">System default</option>
          {devices.map((device) => (
            <option key={device.id} value={device.id}>
              {device.name}
            </option>
          ))}
          {isDisconnected && <option value={selectedId}>Not connected</option>}
        </select>
      </label>
      {devices.length === 0 && (
        <p class="mic-picker-hint">No microphone found. Plug one in and it will show up here.</p>
      )}
    </div>
  );
}

export default MicPicker;
