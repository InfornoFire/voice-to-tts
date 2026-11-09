import { openUrl } from "@tauri-apps/plugin-opener";

import { logFailure } from "./log";

/** Where to get VB-CABLE, which VB-Audio asks to be credited with this address. */
export const VB_CABLE_URL = "https://www.vb-cable.com";

/** Opens `url` in the default browser. */
export function openLink(url: string) {
  openUrl(url).catch(logFailure("Failed to open the link."));
}
