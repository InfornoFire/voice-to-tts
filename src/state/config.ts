import { signal } from "@preact/signals";

import { type Config, commands, unwrap } from "../ipc";

export const config = signal<Config | null>(null);

export async function loadConfig() {
  config.value = await commands.getConfig();
}

export async function saveConfig(next: Config) {
  config.value = await unwrap(commands.updateConfig(next));
}
