import type { InputDevice } from "../ipc";
import SelectField from "./SelectField";

interface MicPickerProps {
  devices: InputDevice[];
  /** The stored device, or `null` for the system default. */
  selectedId: string | null;
  onSelect: (deviceId: string | null) => void;
}

const NO_MICROPHONE_HINT = "No microphone found. Plug one in and it will show up here.";
const CABLE_HINT =
  "VB-CABLE is the microphone, so the voice plays through your default audio output instead of back into VB-CABLE.";

function MicPicker({ devices, selectedId, onSelect }: MicPickerProps) {
  const selected = devices.find((device) => device.id === selectedId);
  const isDisconnected = selectedId !== null && selected === undefined;
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
      hint={devices.length === 0 ? NO_MICROPHONE_HINT : selected?.is_cable ? CABLE_HINT : undefined}
      onChange={(value) => onSelect(value || null)}
    />
  );
}

export default MicPicker;
