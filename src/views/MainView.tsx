import ErrorBanner from "../components/ErrorBanner";
import PowerToggle from "../components/PowerToggle";
import SpeakBox from "../components/SpeakBox";
import StatusLine from "../components/StatusLine";
import Transcript from "../components/Transcript";
import ViewHeader from "../components/ViewHeader";
import {
  activeError,
  cancelSpeech,
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

// The length of the pause is `CANCEL_PAUSE` in `pipeline.rs`.
const CANCEL_HINT = "Stops the voice and ignores the microphone for 3 seconds.";

const openSettings = () => navigate("settings");

function MainView() {
  const error = activeError.value;
  const { status } = engineState.value;
  const typingBlockedReason =
    status === "paused" ? "Paused for a moment" : "Turn the switch on to type a message";

  return (
    <main class="view">
      <ViewHeader title="voice-to-tts" actionLabel="Settings" onAction={openSettings} />
      {error && <ErrorBanner error={error} onAction={runErrorAction} />}
      <div class="main-controls">
        <PowerToggle checked={isEngineOn.value} onToggle={toggleEngine} />
        <StatusLine status={status} />
        <button
          type="button"
          class="button"
          title={CANCEL_HINT}
          disabled={!isEngineRunning.value}
          onClick={cancelSpeech}
        >
          Cancel speech
        </button>
      </div>
      <Transcript utterances={utterances.value} />
      <SpeakBox
        disabled={!isEngineRunning.value}
        disabledReason={typingBlockedReason}
        onSpeak={speakText}
      />
    </main>
  );
}

export default MainView;
