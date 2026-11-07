import { effect, signal } from "@preact/signals";

import { type Transcript, events } from "../ipc";
import { engineState } from "./engine";

export interface Utterance {
  id: number;
  text: string;
  final: boolean;
}

const MAX_UTTERANCES = 200;

export const utterances = signal<Utterance[]>([]);

/** Adds a transcript or updates its utterance. A late partial never undoes a final. */
export function mergeTranscript(
  list: Utterance[],
  transcript: Transcript,
  final: boolean,
): Utterance[] {
  const next = { id: transcript.utterance_id, text: transcript.text, final };
  const index = list.findLastIndex((utterance) => utterance.id === next.id);
  if (index === -1) {
    return [...list, next].slice(-MAX_UTTERANCES);
  }
  if (list[index].final && !final) {
    return list;
  }
  return list.with(index, next);
}

/** Removes utterances that never reached their final text. */
export function withoutPartials(list: Utterance[]): Utterance[] {
  return list.some((utterance) => !utterance.final)
    ? list.filter((utterance) => utterance.final)
    : list;
}

/** Keeps `utterances` in step with the backend. */
export async function syncTranscript() {
  await Promise.all([
    events.transcriptPartial.listen((event) => {
      utterances.value = mergeTranscript(utterances.value, event.payload, false);
    }),
    events.transcriptFinal.listen((event) => {
      utterances.value = mergeTranscript(utterances.value, event.payload, true);
    }),
  ]);

  effect(() => {
    const { status } = engineState.value;
    if (status === "off" || status === "error") {
      utterances.value = withoutPartials(utterances.peek());
    }
  });
}
