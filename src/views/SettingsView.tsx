import { useEffect } from "preact/hooks";

import About from "../components/About";
import CheckboxField from "../components/CheckboxField";
import ErrorBanner from "../components/ErrorBanner";
import HotkeyField from "../components/HotkeyField";
import MicPicker from "../components/MicPicker";
import NumberField from "../components/NumberField";
import RangeField from "../components/RangeField";
import ReadOnlyField from "../components/ReadOnlyField";
import SelectField from "../components/SelectField";
import ViewHeader from "../components/ViewHeader";
import type { CloseAction } from "../ipc";
import { config } from "../state/config";
import { deviceError, inputDevices, isCableInstalled, refreshInputDevices } from "../state/devices";
import { voices } from "../state/models";
import {
  SUBTITLES_PORT,
  THREAD_COUNTS,
  TTS_SPEED,
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

const CLOSE_OPTIONS: { value: CloseAction; label: string }[] = [
  { value: "ask", label: "Ask me" },
  { value: "tray", label: "Minimize to the tray" },
  { value: "quit", label: "Quit" },
];

const HEAR_VOICE_HINT =
  "Also play the voice through your audio output. Without this, only the chat app hears it.";
const ALWAYS_HEARD_HINT =
  "The voice has no virtual microphone to go to here, so it always plays through your audio output.";
const SUBTITLES_HINT = "Lets an OBS browser source show the text the voice speaks.";
const SUBTITLES_ADDRESS_HINT = "In OBS, add a Browser source and paste this address.";

const closeSettings = () => navigate("main");

const openGuide = () => navigate("guide");

const reloadDevices = () => {
  void refreshInputDevices();
};

const formatSpeed = (speed: number) => `${speed.toFixed(1)}×`;

function SettingsView() {
  useEffect(reloadDevices, []);

  const current = config.value;
  const saveFailed = settingsError.value !== null;
  const error = settingsError.value ?? deviceError.value;
  const voiceOptions = voices.value.map((voice) => ({ value: voice.id, label: voice.name }));
  // A stored voice that no longer exists is spoken with the default, which is the first.
  const isKnownVoice = voices.value.some((voice) => voice.id === current?.tts_voice);
  const selectedVoice = isKnownVoice ? current?.tts_voice : voices.value[0]?.id;
  // Without a virtual microphone to receive it, the voice plays through the audio output.
  const isCableMicrophone = inputDevices.value.some(
    (device) => device.id === current?.input_device && device.is_cable,
  );
  const voiceAlwaysHeard = isCableInstalled.value === false || isCableMicrophone;

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
            value={selectedVoice ?? current.tts_voice}
            options={voiceOptions}
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
          <CheckboxField
            label="Hear the voice yourself"
            checked={voiceAlwaysHeard || current.hear_voice}
            disabled={voiceAlwaysHeard}
            hint={voiceAlwaysHeard ? ALWAYS_HEARD_HINT : HEAR_VOICE_HINT}
            onChange={(hearVoice) => updateSettings({ hear_voice: hearVoice })}
          />
          <CheckboxField
            label="Show subtitles in OBS"
            checked={current.subtitles_enabled}
            hint={SUBTITLES_HINT}
            onChange={(enabled) => updateSettings({ subtitles_enabled: enabled })}
          />
          {current.subtitles_enabled && (
            <>
              <NumberField
                label="Subtitles port"
                value={current.subtitles_port}
                min={SUBTITLES_PORT.min}
                max={SUBTITLES_PORT.max}
                onChange={(port) => updateSettings({ subtitles_port: port })}
              />
              <ReadOnlyField
                label="Browser source address"
                value={`http://127.0.0.1:${current.subtitles_port}`}
                hint={SUBTITLES_ADDRESS_HINT}
              />
            </>
          )}
          <SelectField
            label="CPU threads"
            value={String(current.threads)}
            options={THREAD_OPTIONS}
            hint="Fewer threads leave more for other applications, but speech may take longer."
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
          <SelectField
            label="When the window is closed"
            value={current.close_action}
            options={CLOSE_OPTIONS}
            hint="In the tray, the shortcut and the voice keep working."
            onChange={(action) => updateSettings({ close_action: action })}
          />
          <button type="button" class="button settings-guide" onClick={openGuide}>
            Show the setup guide
          </button>
          <About />
        </div>
      )}
    </main>
  );
}

export default SettingsView;
