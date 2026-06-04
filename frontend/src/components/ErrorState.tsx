import "./ErrorState.css";

interface ErrorStateProps {
  title?: string;
  message: string;
  primaryActionLabel?: string;
  onPrimaryAction?: () => void;
}

export default function ErrorState({
  title,
  message,
  primaryActionLabel,
  onPrimaryAction,
}: ErrorStateProps) {
  return (
    <div className="card error-state">
      <div className="error-icon-container">
        <div className="error-icon">
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <circle cx="12" cy="12" r="10" />
            <line x1="12" y1="8" x2="12" y2="12" />
            <line x1="12" y1="16" x2="12.01" y2="16" />
          </svg>
        </div>
      </div>
      <div className="error-content">
        {title && <h1 className="card-title">{title}</h1>}
        <p className="card-text error-message">{message}</p>
      </div>
      {primaryActionLabel && onPrimaryAction && (
        <button className="btn primary" type="button" onClick={onPrimaryAction}>
          <svg
            className="btn-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <polyline points="23 4 23 10 17 10" />
            <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
          </svg>
          {primaryActionLabel}
        </button>
      )}
    </div>
  );
}
