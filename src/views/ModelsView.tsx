import ErrorBanner from "../components/ErrorBanner";
import ProgressBar from "../components/ProgressBar";
import type { ModelsState } from "../ipc";
import { formatSize } from "../lib/format";
import { repairModels } from "../state/engine";
import { cancelDownload } from "../state/models";

import "./ModelsView.css";

interface ModelsViewProps {
  /** Why the speech models cannot be used yet. */
  state: Exclude<ModelsState, { status: "ready" }>;
}

const download = () => {
  void repairModels();
};

function ModelsView({ state }: ModelsViewProps) {
  return (
    <main class="view models-view">
      <h1 class="models-view-title">Download the speech models</h1>
      <p>
        voice-to-tts needs to download its speech models once. After that everything runs on this
        computer, and your voice never leaves it.
      </p>
      {state.status === "missing" && (
        <>
          <button type="button" class="button button-primary" onClick={download}>
            Download ({formatSize(state.bytes)})
          </button>
          <p class="hint">This can take a few minutes.</p>
        </>
      )}
      {state.status === "downloading" && (
        <>
          <ProgressBar
            label="Downloading the speech models"
            value={state.downloaded}
            max={state.total}
          />
          <button type="button" class="button" onClick={cancelDownload}>
            Cancel
          </button>
        </>
      )}
      {state.status === "failed" && <ErrorBanner error={state.error} onAction={download} />}
    </main>
  );
}

export default ModelsView;
