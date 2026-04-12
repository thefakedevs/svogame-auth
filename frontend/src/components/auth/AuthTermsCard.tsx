import { useState } from 'react'
import { paths } from '../../routes/paths'
import './AuthTermsCard.css'

export default function AuthTermsCard({
  onAccept,
  onRestart,
  errorMessage,
  isSubmitting,
}: {
  onAccept: () => void
  onRestart: () => void
  errorMessage?: string
  isSubmitting?: boolean
}) {
  const [accepted, setAccepted] = useState(false)

  return (
    <div className="page auth-start-page auth-pow-fullbleed">
      <div className="ui-kit-vhs" aria-hidden />
      <div className="ui-kit-page">
        <section className="card auth-terms-card">
          <h1 className="card-title">Подтвердите условия перед созданием аккаунта</h1>
          <p className="card-text">
            Этот шаг нужен только при первой регистрации. Для уже существующих аккаунтов мы его не показываем.
          </p>
          <label className="auth-terms-card__checkbox ui-checkbox">
            <input
              type="checkbox"
              checked={accepted}
              onChange={(event) => setAccepted(event.target.checked)}
              disabled={isSubmitting}
            />
            <span className="ui-checkbox-mark" aria-hidden />
            <span>
              Принимаю{' '}
              <a href={paths.legalUserAgreement} target="_blank" rel="noreferrer">пользовательское соглашение</a>
              {' '}и{' '}
              <a href={paths.legalPrivacyPolicy} target="_blank" rel="noreferrer">политику конфиденциальности</a>
              . <br/>Полный список документов доступен в{' '}
              <a href={paths.legal} target="_blank" rel="noreferrer">legal-разделе</a>
              .
            </span>
          </label>
          <p className="auth-terms-card__hint">
            Аккаунт будет создан только после подтверждения.
          </p>
          {errorMessage ? <p className="card-text">{errorMessage}</p> : null}
          <div className="auth-terms-card__actions">
            <button className="btn" type="button" onClick={onRestart} disabled={isSubmitting}>
              Начать заново
            </button>
            <button
              className="btn primary"
              type="button"
              onClick={onAccept}
              disabled={!accepted || isSubmitting}
            >
              {isSubmitting ? 'Создаём аккаунт...' : 'Принять и продолжить'}
            </button>
          </div>
        </section>
      </div>
    </div>
  )
}
