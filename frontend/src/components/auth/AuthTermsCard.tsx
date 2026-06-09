import { useState } from 'react'
import type { AuthReferralState } from '../../features/auth/hooks/useAuthFlow'
import type { ReferralRewardResponse } from '../../api/referrals'
import { paths } from '../../routes/paths'
import './AuthTermsCard.css'

function formatReward(reward: ReferralRewardResponse): string {
  const name = reward.assetDisplayName || reward.assetKey
  if (reward.amount && reward.amount > 0) {
    return `${name} x${reward.amount}`
  }
  if (reward.durationSeconds && reward.durationSeconds > 0) {
    const hours = Math.round(reward.durationSeconds / 3600)
    return `${name}, ${hours > 0 ? `${hours} ч` : `${reward.durationSeconds} сек`}`
  }
  return name
}

export default function AuthTermsCard({
  onAccept,
  onRestart,
  referral,
  onReferralInputChange,
  onApplyReferral,
  onClearReferral,
  errorMessage,
  isSubmitting,
}: {
  onAccept: () => void
  onRestart: () => void
  referral: AuthReferralState
  onReferralInputChange: (value: string) => void
  onApplyReferral: () => void
  onClearReferral: () => void
  errorMessage?: string
  isSubmitting?: boolean
}) {
  const [accepted, setAccepted] = useState(false)
  const isLinkReferral = referral.candidate?.source === 'link'
  const showManualInput = !isLinkReferral || referral.status === 'unavailable'
  const isCheckingReferral = referral.status === 'loading'

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
          <section className="auth-referral-card" aria-label="Реферальный код">
            <div className="auth-referral-card__head">
              <div>
                <h2>Welcome pack</h2>
                <p>
                  {referral.preview
                    ? 'Этот welcome pack будет запрошен при создании аккаунта.'
                    : isLinkReferral
                    ? 'Проверяем реферальный код из ссылки.'
                    : 'Если у вас есть реферальный код, добавьте его перед созданием аккаунта.'}
                </p>
              </div>
              {referral.preview ? (
                <span className="ui-badge ui-badge-success">Код применится</span>
              ) : isCheckingReferral ? (
                <span className="ui-badge ui-badge-neutral">Проверка</span>
              ) : null}
            </div>

            {referral.preview ? (
              <div className="auth-referral-preview">
                <div>
                  <strong>{referral.preview.title}</strong>
                  <small>{referral.preview.code}</small>
                </div>
                {referral.preview.rewards.length > 0 ? (
                  <ul>
                    {referral.preview.rewards.map((reward) => (
                      <li key={`${reward.assetKey}-${reward.amount ?? 'once'}-${reward.durationSeconds ?? 'permanent'}`}>
                        {formatReward(reward)}
                      </li>
                    ))}
                  </ul>
                ) : (
                  <p>Кампания активна. Награды не указаны.</p>
                )}
                <button className="btn btn-sm" type="button" onClick={onClearReferral} disabled={isSubmitting}>
                  Убрать код
                </button>
              </div>
            ) : null}

            {!referral.preview && referral.message ? (
              <p className={referral.candidate?.source === 'manual' ? 'auth-referral-error' : 'auth-referral-note'}>
                {referral.message}
              </p>
            ) : null}

            {showManualInput ? (
              <div className="auth-referral-manual">
                <label>
                  <span>Есть реферальный код?</span>
                  <input
                    className="ui-input"
                    value={referral.inputValue}
                    onChange={(event) => onReferralInputChange(event.target.value)}
                    placeholder="CREATOR-ONE"
                    disabled={isSubmitting || isCheckingReferral}
                    spellCheck={false}
                  />
                </label>
                <button
                  className="btn btn-sm"
                  type="button"
                  onClick={onApplyReferral}
                  disabled={isSubmitting || isCheckingReferral}
                >
                  {isCheckingReferral ? 'Проверяем...' : 'Проверить код'}
                </button>
                {referral.inputValue ? (
                  <button className="btn btn-sm" type="button" onClick={onClearReferral} disabled={isSubmitting}>
                    Очистить
                  </button>
                ) : null}
              </div>
            ) : null}
          </section>
          {errorMessage ? <p className="card-text">{errorMessage}</p> : null}
          <div className="auth-terms-card__actions">
            <button className="btn" type="button" onClick={onRestart} disabled={isSubmitting}>
              Начать заново
            </button>
            <button
              className="btn primary"
              type="button"
              onClick={onAccept}
              disabled={!accepted || isSubmitting || isCheckingReferral}
            >
              {isSubmitting ? 'Создаём аккаунт...' : 'Принять и продолжить'}
            </button>
          </div>
        </section>
      </div>
    </div>
  )
}
