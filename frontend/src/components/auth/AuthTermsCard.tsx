import { useEffect, useMemo, useState } from 'react'
import type { AuthReferralState } from '../../features/auth/hooks/useAuthFlow'
import { buildPublicAssetImageUrl, listPublicAssets, type AssetResponse } from '../../api/inventory'
import type { ReferralRewardResponse } from '../../api/referrals'
import { paths } from '../../routes/paths'
import './AuthTermsCard.css'

type ReferralRewardAssetView = {
  title: string
  imageUrl: string | null
}

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

function metadataRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === 'object' && !Array.isArray(value) ? value as Record<string, unknown> : null
}

function metadataString(source: { metadata?: unknown } | null, keys: string[]) {
  const metadata = metadataRecord(source?.metadata)
  if (!metadata) return null

  for (const key of keys) {
    const value = metadata[key]
    if (typeof value === 'string' && value.trim()) return value.trim()
  }

  return null
}

function rewardAssetImageUrl(asset: AssetResponse | null) {
  if (!asset) return null

  return asset.imageUrl
    ?? metadataString(asset, ['imageUrl', 'image', 'iconUrl', 'icon', 'thumbnailUrl', 'thumbnail', 'previewUrl', 'skinUrl'])
    ?? buildPublicAssetImageUrl(asset.id, asset.updatedAt)
}

function rewardAssetFallback(value: string) {
  return value.trim().slice(0, 1).toUpperCase() || '?'
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
  const [rewardAssets, setRewardAssets] = useState<Map<string, ReferralRewardAssetView>>(() => new Map())
  const isLinkReferral = referral.candidate?.source === 'link'
  const showManualInput = !isLinkReferral || referral.status === 'unavailable'
  const isCheckingReferral = referral.status === 'loading'
  const rewardAssetKeys = useMemo(
    () => Array.from(new Set(referral.preview?.rewards.map((reward) => reward.assetKey) ?? [])),
    [referral.preview],
  )

  useEffect(() => {
    if (rewardAssetKeys.length === 0) {
      setRewardAssets(new Map())
      return
    }

    let cancelled = false

    const load = async () => {
      const entries = await Promise.all(
        rewardAssetKeys.map(async (assetKey): Promise<[string, ReferralRewardAssetView | null]> => {
          try {
            const response = await listPublicAssets({ q: assetKey, perPage: 20 })
            const normalizedKey = assetKey.toLowerCase()
            const asset = response.items.find((item) => item.key.toLowerCase() === normalizedKey) ?? null

            return [
              assetKey,
              asset
                ? {
                    title: asset.displayName,
                    imageUrl: rewardAssetImageUrl(asset),
                  }
                : null,
            ]
          } catch {
            return [assetKey, null]
          }
        }),
      )

      if (cancelled) return

      const nextAssets = new Map<string, ReferralRewardAssetView>()
      for (const [assetKey, asset] of entries) {
        if (asset) nextAssets.set(assetKey, asset)
      }
      setRewardAssets(nextAssets)
    }

    void load()

    return () => {
      cancelled = true
    }
  }, [rewardAssetKeys])

  return (
    <div className="page auth-start-page auth-pow-fullbleed">
      <div className="ui-kit-vhs" aria-hidden />
      <div className="ui-kit-page auth-terms-page">
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
                <h2>Бонус за приглашение</h2>
                <p>
                  {referral.preview
                    ? isLinkReferral
                      ? 'Код приглашения уже привязан к регистрации. После создания аккаунта останется подготовиться к первой игре.'
                      : 'Реферальный бонус будет добавлен к заявке на создание аккаунта.'
                    : isLinkReferral
                    ? 'Проверяем код приглашения из ссылки.'
                    : 'Если вас пригласили на сервер, введите код приглашения. Если кода нет, просто продолжайте регистрацию.'}
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
                </div>
                {referral.preview.rewards.length > 0 ? (
                  <ul>
                    {referral.preview.rewards.map((reward) => {
                      const rewardAsset = rewardAssets.get(reward.assetKey) ?? null
                      const title = rewardAsset?.title ?? reward.assetDisplayName ?? reward.assetKey

                      return (
                        <li key={`${reward.assetKey}-${reward.amount ?? 'once'}-${reward.durationSeconds ?? 'permanent'}`}>
                          <span className="auth-referral-reward-image" aria-hidden>
                            {rewardAsset?.imageUrl ? (
                              <img src={rewardAsset.imageUrl} alt="" loading="lazy" />
                            ) : (
                              <span>{rewardAssetFallback(title)}</span>
                            )}
                          </span>
                          <span>{formatReward(reward)}</span>
                        </li>
                      )
                    })}
                  </ul>
                ) : (
                  <p>Код активен. Бонусы для этой кампании не указаны.</p>
                )}
                <button className="btn btn-sm" type="button" onClick={onClearReferral} disabled={isSubmitting}>
                  Убрать код
                </button>
              </div>
            ) : null}

            {isLinkReferral && referral.preview ? (
              <div className="auth-referral-checklist" aria-label="Что сделать после регистрации">
                <strong>После регистрации</strong>
                <ol>
                  <li>Скачайте лаунчер и войдите в аккаунт.</li>
                  <li>Зайдите в Discord, чтобы не пропустить анонсы игр.</li>
                  <li>Дождитесь ближайшего ивента.</li>
                </ol>
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
                  <span>Код приглашения, если он у вас есть</span>
                  <input
                    className="ui-input"
                    value={referral.inputValue}
                    onChange={(event) => onReferralInputChange(event.target.value)}
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
