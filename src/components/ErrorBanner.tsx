import type { ErrorAction, UserFacingError } from "../ipc";

import "./ErrorBanner.css";

const ACTION_LABELS: Record<ErrorAction, string> = {
  repair: "Repair",
  install_cable: "Get VB-CABLE",
  restart: "Restart",
  pick_mic: "Pick a microphone",
  retry: "Try again",
};

interface ErrorBannerProps {
  error: UserFacingError;
  onAction: (action: ErrorAction) => void;
}

function ErrorBanner({ error, onAction }: ErrorBannerProps) {
  const { action } = error;

  return (
    <div class="error-banner" role="alert">
      <p class="error-banner-message">{error.message}</p>
      {action && (
        <button type="button" class="button" onClick={() => onAction(action)}>
          {ACTION_LABELS[action]}
        </button>
      )}
    </div>
  );
}

export default ErrorBanner;
