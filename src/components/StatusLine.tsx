import type { EngineState } from "../ipc";

import "./StatusLine.css";

type Status = EngineState["status"];

const LABELS: Record<Status, string> = {
  off: "Off",
  loading: "Getting ready…",
  listening: "Listening",
  speaking: "Speaking",
  error: "Needs attention",
};

interface StatusLineProps {
  status: Status;
}

function StatusLine({ status }: StatusLineProps) {
  return (
    <p class="status-line" data-status={status} aria-live="polite">
      {LABELS[status]}
    </p>
  );
}

export default StatusLine;
