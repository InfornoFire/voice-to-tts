import ErrorBanner from "../components/ErrorBanner";
import PowerToggle from "../components/PowerToggle";
import SpeakBox from "../components/SpeakBox";
import StatusLine from "../components/StatusLine";
import Transcript from "../components/Transcript";
import ViewHeader from "../components/ViewHeader";
import {
  activeError,
  canToggleEngine,
  engineState,
  isEngineOn,
  isEngineRunning,
  runErrorAction,
  speakText,
  toggleEngine,
} from "../state/engine";
import { utterances } from "../state/transcript";
import { navigate } from "../state/view";

import "./MainView.css";

const openSettings = () => navigate("settings");

function MainView() {
  const error = activeError.value;

  return (
    <main class="view">
      <ViewHeader title="voice-to-tts" actionLabel="Settings" onAction={openSettings} />
      {error && <ErrorBanner error={error} onAction={runErrorAction} />}
      <div class="main-controls">
        <PowerToggle
          checked={isEngineOn.value}
          disabled={!canToggleEngine.value}
          onToggle={toggleEngine}
        />
        <StatusLine status={engineState.value.status} />
      </div>
      <Transcript utterances={utterances.value} />
      <SpeakBox disabled={!isEngineRunning.value} onSpeak={speakText} />
    </main>
  );
}

export default MainView;
