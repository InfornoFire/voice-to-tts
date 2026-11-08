import { signal } from "@preact/signals";

export type View = "main" | "settings" | "guide";

export const currentView = signal<View>("main");

export function navigate(view: View) {
  currentView.value = view;
}
