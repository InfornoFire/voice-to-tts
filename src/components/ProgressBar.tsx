import { formatSize } from "../lib/format";

import "./ProgressBar.css";

interface ProgressBarProps {
  label: string;
  /** Bytes done so far. */
  value: number;
  /** Bytes in total. */
  max: number;
}

function ProgressBar({ label, value, max }: ProgressBarProps) {
  return (
    <div class="progress">
      <progress class="progress-bar" aria-label={label} value={value} max={Math.max(max, 1)} />
      <p class="hint">
        {formatSize(value)} of {formatSize(max)}
      </p>
    </div>
  );
}

export default ProgressBar;
