import ErrorBanner from "../components/ErrorBanner";
import GuideStep from "../components/GuideStep";
import PowerToggle from "../components/PowerToggle";
import SignalPath from "../components/SignalPath";
import StatusLine from "../components/StatusLine";
import ViewHeader from "../components/ViewHeader";
import { VB_CABLE_URL, openLink } from "../lib/links";
import { isCableInstalled } from "../state/devices";
import {
  activeError,
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
          voice-to-tts says what you say in a computer voice. It plays the voice through your audio
          output, or, with a virtual microphone, your chat app hears that voice instead of yours. It
          takes three steps to set up.
        </p>
        <GuideStep number={1} title="Install the virtual microphone (optional)">
          {isCableInstalled.value === null && <p class="hint">Checking for VB-CABLE…</p>}
          {isCableInstalled.value === true && <p>VB-CABLE is installed.</p>}
          {isCableInstalled.value === false && (
            <>
              <p>
                VB-CABLE is not installed. Without it the voice plays through your audio output. To
                use it with a chat app, download VB-CABLE, run its setup as administrator, then
                restart your computer.
              </p>
              <button type="button" class="button guide-action" onClick={getCable}>
                Get VB-CABLE
              </button>
            </>
          )}
        </GuideStep>
        <GuideStep number={2} title="Choose it in your chat app">
          <p class="hint">Only needed with VB-CABLE.</p>
          <p>
            Open the audio settings of your destination chat app and set its microphone (input
            device) to <strong>CABLE Output</strong>. Do not pick your real microphone, or people
            will hear your real voice.
          </p>
          <SignalPath />
        </GuideStep>
        <GuideStep number={3} title="Try it">
          <p>
            {isCableInstalled.value === false
              ? "Turn the switch on and play the test sentence. You will hear it through your audio output."
              : "Turn the switch on, play the test sentence, and watch the input level of your chat app move."}
          </p>
          <div class="guide-try">
            <PowerToggle checked={isEngineOn.value} onToggle={toggleEngine} />
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
          {error && <ErrorBanner error={error} onAction={runErrorAction} />}
        </GuideStep>
        <button type="button" class="button button-primary guide-action" onClick={finishGuide}>
          Done
        </button>
      </div>
    </main>
  );
}

export default GuideView;
