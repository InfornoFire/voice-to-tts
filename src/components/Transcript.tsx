import { useStickToBottom } from "../hooks/use-stick-to-bottom";
import type { Utterance } from "../state/transcript";

import "./Transcript.css";

interface TranscriptProps {
  utterances: Utterance[];
}

function Transcript({ utterances }: TranscriptProps) {
  const { ref, onScroll } = useStickToBottom(utterances);

  return (
    <section class="transcript" aria-label="Transcript" ref={ref} onScroll={onScroll}>
      {utterances.length === 0 ? (
        <p class="transcript-empty">What you say will show up here.</p>
      ) : (
        <ol class="transcript-list">
          {utterances.map((utterance) => (
            <li
              key={utterance.id}
              class="transcript-line"
              data-state={utterance.final ? "final" : "partial"}
            >
              {utterance.text}
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}

export default Transcript;
