import { openUrl } from "@tauri-apps/plugin-opener";

/** Where to get VB-CABLE, which VB-Audio asks to be credited with this address. */
export const VB_CABLE_URL = "https://www.vb-cable.com";

/** Opens `url` in the default browser. */
export function openLink(url: string) {
  openUrl(url).catch((error: unknown) => console.error("Failed to open the link.", error));
}
