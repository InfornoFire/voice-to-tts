import "./ViewHeader.css";

interface ViewHeaderProps {
  title: string;
  actionLabel: string;
  onAction: () => void;
}

function ViewHeader({ title, actionLabel, onAction }: ViewHeaderProps) {
  return (
    <header class="view-header">
      <h1 class="view-header-title">{title}</h1>
      <button type="button" class="view-header-action" onClick={onAction}>
        {actionLabel}
      </button>
    </header>
  );
}

export default ViewHeader;
