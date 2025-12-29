interface ErrorStateProps {
  title?: string
  message: string
  primaryActionLabel?: string
  onPrimaryAction?: () => void
}

export default function ErrorState({ title, message, primaryActionLabel, onPrimaryAction }: ErrorStateProps) {
  return (
    <div className="card error-state">
      {title && <h1 className="card-title">{title}</h1>}
      <p className="card-text">{message}</p>
      {primaryActionLabel && onPrimaryAction && (
        <button className="btn" type="button" onClick={onPrimaryAction}>
          {primaryActionLabel}
        </button>
      )}
    </div>
  )
}

