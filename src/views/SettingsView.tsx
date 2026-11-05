import { useEffect } from "preact/hooks";

import ErrorBanner from "../components/ErrorBanner";
import MicPicker from "../components/MicPicker";
import ViewHeader from "../components/ViewHeader";
import { config } from "../state/config";
import {
  deviceError,
  inputDevices,
  refreshInputDevices,
  selectInputDevice,
} from "../state/devices";
import { navigate } from "../state/view";

const closeSettings = () => navigate("main");

const chooseMic = (deviceId: string | null) => {
  void selectInputDevice(deviceId);
};

const reloadDevices = () => {
  void refreshInputDevices();
};

function SettingsView() {
  useEffect(reloadDevices, []);

  const error = deviceError.value;
  const current = config.value;

  return (
    <main class="view">
      <ViewHeader title="Settings" actionLabel="Back" onAction={closeSettings} />
      {error && <ErrorBanner error={error} onAction={reloadDevices} />}
      <MicPicker
        devices={inputDevices.value}
        selectedId={current?.input_device ?? null}
        disabled={current === null}
        onSelect={chooseMic}
      />
    </main>
  );
}

export default SettingsView;
