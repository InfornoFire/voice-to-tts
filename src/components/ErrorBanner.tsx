import type { ErrorAction, UserFacingError } from "../ipc";

import "./ErrorBanner.css";

const ACTION_LABELS: Record<ErrorAction, string> = {
  repair: "Repair",
  pick_mic: "Pick a microphone",
  retry: "Try again",
};

interface ErrorBannerProps {
  error: UserFacingError;
  /** Called with `error` when its action button is pressed. */
  onAction: (error: UserFacingError) => void;
}

function ErrorBanner({ error, onAction }: ErrorBannerProps) {
  const { action } = error;

  return (
    <div class="error-banner" role="alert">
      <p class="error-banner-message">{error.message}</p>
      {action && (
        <button type="button" class="button" onClick={() => onAction(error)}>
          {ACTION_LABELS[action]}
        </button>
      )}
    </div>
  );
}

export default ErrorBanner;
