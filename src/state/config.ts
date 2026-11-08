import { signal } from "@preact/signals";

import { type Config, commands } from "../ipc";
import { navigate } from "./view";

export const config = signal<Config | null>(null);

/** Loads the config, and opens the setup guide for someone who has not been through it. */
export async function loadConfig() {
  const loaded = await commands.getConfig();
  config.value = loaded;
  if (!loaded.setup_guide_done) {
    navigate("guide");
  }
}
