import { signal } from "@preact/signals";

import { type Config, commands } from "../ipc";

export const config = signal<Config | null>(null);

export async function loadConfig() {
  config.value = await commands.getConfig();
}
