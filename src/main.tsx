import { render } from "preact";

import App from "./App";
import { loadConfig } from "./state/config";
import { syncDevices } from "./state/devices";
import { syncEngineState } from "./state/engine";
import { syncTranscript } from "./state/transcript";

import "./styles/tokens.css";
import "./styles/base.css";

const logFailure = (message: string) => (error: unknown) => console.error(message, error);

render(<App />, document.getElementById("root")!);

loadConfig().catch(logFailure("Failed to load the config."));
syncEngineState().catch(logFailure("Failed to sync the engine state."));
syncTranscript().catch(logFailure("Failed to sync the transcript."));
syncDevices();
