export default function UiSpinner({ size = 'md' }: { size?: 'sm' | 'md' | 'lg' }) {
  const sizeClass = size === 'sm' ? 'ui-spinner-sm' : size === 'lg' ? 'ui-spinner-lg' : ''
  return (
    <span className={['ui-spinner', sizeClass].filter(Boolean).join(' ')} role="status" aria-label="Загрузка">
      <span className="ui-spinner-track" aria-hidden>
        <span className="ui-spinner-orb" />
        <span className="ui-spinner-orb" />
        <span className="ui-spinner-orb" />
        <span className="ui-spinner-orb" />
      </span>
    </span>
  )
}
