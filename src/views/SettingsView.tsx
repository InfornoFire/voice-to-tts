import { useEffect } from "preact/hooks";

import CheckboxField from "../components/CheckboxField";
import ErrorBanner from "../components/ErrorBanner";
import HotkeyField from "../components/HotkeyField";
import MicPicker from "../components/MicPicker";
import RangeField from "../components/RangeField";
import SelectField from "../components/SelectField";
import ViewHeader from "../components/ViewHeader";
import { config } from "../state/config";
import { deviceError, inputDevices, refreshInputDevices } from "../state/devices";
import {
  THREAD_COUNTS,
  TTS_SPEED,
  VOICES,
  pauseHotkey,
  saveSettings,
  settingsError,
  updateSettings,
} from "../state/settings";
import { navigate } from "../state/view";

import "./SettingsView.css";

const THREAD_OPTIONS = THREAD_COUNTS.map((count) => ({
  value: String(count),
  label: String(count),
}));

const closeSettings = () => navigate("main");

const reloadDevices = () => {
  void refreshInputDevices();
};

const formatSpeed = (speed: number) => `${speed.toFixed(1)}×`;

function SettingsView() {
  useEffect(reloadDevices, []);

  const current = config.value;
  const saveFailed = settingsError.value !== null;
  const error = settingsError.value ?? deviceError.value;

  return (
    <main class="view">
      <ViewHeader title="Settings" actionLabel="Back" onAction={closeSettings} />
      {error && <ErrorBanner error={error} onAction={saveFailed ? saveSettings : reloadDevices} />}
      {current && (
        <div class="settings-list">
          <MicPicker
            devices={inputDevices.value}
            selectedId={current.input_device}
            onSelect={(deviceId) => updateSettings({ input_device: deviceId })}
          />
          <SelectField
            label="Voice"
            value={current.tts_voice}
            options={VOICES}
            onChange={(voice) => updateSettings({ tts_voice: voice })}
          />
          <RangeField
            label="Speaking speed"
            value={current.tts_speed ?? TTS_SPEED.fallback}
            min={TTS_SPEED.min}
            max={TTS_SPEED.max}
            step={TTS_SPEED.step}
            format={formatSpeed}
            onChange={(speed) => updateSettings({ tts_speed: speed })}
          />
          <SelectField
            label="CPU threads"
            value={String(current.threads)}
            options={THREAD_OPTIONS}
            hint="Fewer threads leave more power for your games and calls, but speech may take longer."
            onChange={(count) => updateSettings({ threads: Number(count) })}
          />
          <HotkeyField
            label="Shortcut to switch on and off"
            value={current.hotkey}
            onChange={(hotkey) => updateSettings({ hotkey })}
            onRecordingChange={pauseHotkey}
          />
          <CheckboxField
            label="Start minimized"
            checked={current.start_minimized}
            hint="Open in the background when the app starts."
            onChange={(startMinimized) => updateSettings({ start_minimized: startMinimized })}
          />
        </div>
      )}
    </main>
  );
}

export default SettingsView;
