import { render } from "preact";

import App from "./App";
import { loadConfig } from "./state/config";

import "./styles/tokens.css";
import "./styles/base.css";

render(<App />, document.getElementById("root")!);

loadConfig().catch((error: unknown) => console.error("Failed to load the config.", error));
