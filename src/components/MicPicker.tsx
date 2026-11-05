import type { InputDevice } from "../ipc";
import SelectField from "./SelectField";

interface MicPickerProps {
  devices: InputDevice[];
  /** The stored device, or `null` for the system default. */
  selectedId: string | null;
  onSelect: (deviceId: string | null) => void;
}

function MicPicker({ devices, selectedId, onSelect }: MicPickerProps) {
  const isDisconnected = selectedId !== null && !devices.some((device) => device.id === selectedId);
  const options = [
    { value: "", label: "System default" },
    ...devices.map((device) => ({ value: device.id, label: device.name })),
    ...(isDisconnected ? [{ value: selectedId, label: "Not connected" }] : []),
  ];

  return (
    <SelectField
      label="Microphone"
      value={selectedId ?? ""}
      options={options}
      hint={
        devices.length === 0
          ? "No microphone found. Plug one in and it will show up here."
          : undefined
      }
      onChange={(value) => onSelect(value || null)}
    />
  );
}

export default MicPicker;
