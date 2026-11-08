import ErrorBanner from "../components/ErrorBanner";
import GuideStep from "../components/GuideStep";
import PowerToggle from "../components/PowerToggle";
import SignalPath from "../components/SignalPath";
import StatusLine from "../components/StatusLine";
import ViewHeader from "../components/ViewHeader";
import { VB_CABLE_URL, openLink } from "../lib/links";
import { cableError, isCableChecked } from "../state/devices";
import {
  activeError,
  canToggleEngine,
  engineState,
  isEngineOn,
  isEngineRunning,
  playTestSentence,
  runErrorAction,
  toggleEngine,
} from "../state/engine";
import { finishGuide } from "../state/settings";

import "./GuideView.css";

const getCable = () => openLink(VB_CABLE_URL);

const playTest = () => {
  void playTestSentence();
};

function GuideView() {
  const error = activeError.value;

  return (
    <main class="view">
      <ViewHeader title="Getting started" actionLabel="Skip" onAction={finishGuide} />
      <div class="guide-steps">
        <p class="guide-intro">
          voice-to-tts says what you say in a computer voice, through a virtual microphone. Your
          chat app hears that voice instead of yours. It takes three steps to set up.
        </p>
        <GuideStep number={1} title="Install the virtual microphone">
          {!isCableChecked.value && <p class="hint">Checking for VB-CABLE…</p>}
          {isCableChecked.value && cableError.value === null && <p>VB-CABLE is installed.</p>}
          {cableError.value !== null && (
            <>
              <p>
                VB-CABLE is not installed yet. Download it, run its setup as administrator, then
                restart your computer.
              </p>
              <button type="button" class="button guide-action" onClick={getCable}>
                Get VB-CABLE
              </button>
            </>
          )}
        </GuideStep>
        <GuideStep number={2} title="Choose it in your chat app">
          <p>
            Open the audio settings of your destination chat app and set its microphone (input
            device) to <strong>CABLE Output</strong>. Do not pick your real microphone, or people
            will hear your real voice.
          </p>
          <SignalPath />
        </GuideStep>
        <GuideStep number={3} title="Try it">
          <p>
            Turn the switch on, play the test sentence, and watch the input level of your chat app
            move.
          </p>
          <div class="guide-try">
            <PowerToggle
              checked={isEngineOn.value}
              disabled={!canToggleEngine.value}
              onToggle={toggleEngine}
            />
            <StatusLine status={engineState.value.status} />
            <button
              type="button"
              class="button"
              disabled={!isEngineRunning.value}
              onClick={playTest}
            >
              Play test sentence
            </button>
          </div>
          {error && (
            <ErrorBanner error={error} onAction={(action) => runErrorAction(action, error.code)} />
          )}
        </GuideStep>
        <button type="button" class="button button-primary guide-action" onClick={finishGuide}>
          Done
        </button>
      </div>
    </main>
  );
}

export default GuideView;
