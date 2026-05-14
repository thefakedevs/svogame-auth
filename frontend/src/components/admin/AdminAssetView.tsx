import { useEffect, useRef, useState, type ChangeEvent } from 'react'
import toast from 'react-hot-toast'
import {
  buildPublicAssetImageUrl,
  deleteAdminAssetImage,
  getAdminAsset,
  patchAdminAsset,
  uploadAdminAssetImage,
  type AssetResponse,
  type SkinRarity,
} from '../../api/inventory'
import { toDisplayError } from '../../api/http'
import { paths } from '../../routes/paths'
import ErrorState from '../ErrorState'
import LoadingState from '../LoadingState'
import AdminAssetImage from './AdminAssetImage'
import AdminLink from './AdminLink'

type EditAssetDraft = {
  displayName: string
  description: string
  rarity: SkinRarity | ''
  weaponKey: string
  isActive: boolean
  isPublic: boolean
  isUserPurchasable: boolean
  metadataText: string
}

function metadataToText(value: unknown) {
  if (value == null) return '{}'
  try {
    return JSON.stringify(value, null, 2)
  } catch {
    return String(value)
  }
}

function parseMetadata(value: string) {
  const trimmed = value.trim()
  if (!trimmed) return null
  return JSON.parse(trimmed) as unknown
}

function makeEditDraft(asset: AssetResponse): EditAssetDraft {
  return {
    displayName: asset.displayName,
    description: asset.description ?? '',
    rarity: asset.rarity ?? '',
    weaponKey: asset.weaponKey ?? '',
    isActive: asset.isActive,
    isPublic: asset.isPublic,
    isUserPurchasable: asset.isUserPurchasable,
    metadataText: metadataToText(asset.metadata),
  }
}

export default function AdminAssetView({ token, assetId }: { token: string; assetId: string }) {
  const [asset, setAsset] = useState<AssetResponse | null>(null)
  const [draft, setDraft] = useState<EditAssetDraft | null>(null)
  const [loadError, setLoadError] = useState('')
  const [isSaving, setIsSaving] = useState(false)
  const [isImageMutating, setIsImageMutating] = useState(false)
  const [imageReloadKey, setImageReloadKey] = useState(0)
  const imageInputRef = useRef<HTMLInputElement | null>(null)

  useEffect(() => {
    let cancelled = false
    const run = async () => {
      setLoadError('')
      setAsset(null)
      setDraft(null)
      try {
        const loaded = await getAdminAsset(token, assetId)
        if (cancelled) return
        setAsset(loaded)
        setDraft(makeEditDraft(loaded))
      } catch (cause) {
        if (!cancelled) setLoadError(toDisplayError(cause, 'Не удалось загрузить ассет.'))
      }
    }
    void run()
    return () => {
      cancelled = true
    }
  }, [assetId, token])

  const updateAsset = (updated: AssetResponse) => {
    setAsset(updated)
    setDraft(makeEditDraft(updated))
  }

  const uploadImage = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0] ?? null
    event.target.value = ''

    if (!asset || !file || isImageMutating) return
    if (!file.type.startsWith('image/')) {
      toast.error('Выберите файл изображения.')
      return
    }
    if (file.size > 2 * 1024 * 1024) {
      toast.error('Изображение должно быть не больше 2 МБ.')
      return
    }

    setIsImageMutating(true)
    try {
      const updated = await uploadAdminAssetImage(token, asset.id, file)
      updateAsset(updated)
      setImageReloadKey((value) => value + 1)
      toast.success('Изображение ассета загружено.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось загрузить изображение ассета.'))
    } finally {
      setIsImageMutating(false)
    }
  }

  const deleteImage = async () => {
    if (!asset || isImageMutating) return

    setIsImageMutating(true)
    try {
      const updated = await deleteAdminAssetImage(token, asset.id)
      updateAsset(updated)
      setImageReloadKey((value) => value + 1)
      toast.success('Изображение ассета удалено.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось удалить изображение ассета.'))
    } finally {
      setIsImageMutating(false)
    }
  }

  const saveAsset = async () => {
    if (!asset || !draft || isSaving) return
    if (!draft.displayName.trim()) {
      toast.error('Название ассета не может быть пустым.')
      return
    }

    let metadata: unknown
    try {
      metadata = parseMetadata(draft.metadataText)
    } catch {
      toast.error('Metadata должен быть валидным JSON.')
      return
    }

    setIsSaving(true)
    try {
      const updated = await patchAdminAsset(token, asset.id, {
        display_name: draft.displayName.trim(),
        description: draft.description.trim() || null,
        is_active: draft.isActive,
        is_public: draft.isPublic,
        is_user_purchasable: draft.isUserPurchasable,
        ...(asset.assetKind === 'skin'
          ? {
              rarity: draft.rarity || null,
              weaponKey: draft.weaponKey.trim() || null,
            }
          : {}),
        metadata,
      })
      updateAsset(updated)
      toast.success('Ассет обновлен.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось обновить ассет.'))
    } finally {
      setIsSaving(false)
    }
  }

  return (
    <div className="admin-page">
      <section className="admin-top-actions">
        <AdminLink className="btn btn-sm" href={`${paths.admin}?tab=assets`}>
          ← К ассетам
        </AdminLink>
      </section>

      <section className="card admin-card">
        {!asset && !loadError ? <LoadingState title="Загружаем ассет" /> : null}
        {loadError ? <ErrorState message={loadError} /> : null}
        {asset && draft ? (
          <>
            <div className="admin-section-head admin-detail-hero">
              <div>
                <h2 className="card-title">{asset.displayName}</h2>
                <p className="card-text">
                  <code>{asset.key}</code> · ID: <code>{asset.id}</code>
                </p>
              </div>
              <div className="admin-asset-badges">
                <span className="ui-badge ui-badge-neutral">{asset.assetKind}</span>
                <span className="ui-badge ui-badge-neutral">{asset.ownershipModel}</span>
                {asset.isCurrency ? <span className="ui-badge ui-badge-secondary">currency</span> : null}
                <span className={`ui-badge ${asset.isActive ? 'ui-badge-success' : 'ui-badge-warning'}`}>
                  {asset.isActive ? 'active' : 'inactive'}
                </span>
              </div>
            </div>

            <article className="admin-asset-card admin-spaced-form">
              <div className="admin-asset-image-panel">
                <AdminAssetImage token={token} asset={asset} reloadKey={imageReloadKey} />
                <div className="admin-asset-image-info">
                  <strong>Изображение ассета</strong>
                  <small>PNG, JPG или WebP до 2 МБ.</small>
                  <div className="admin-asset-image-actions">
                    <input
                      ref={imageInputRef}
                      className="admin-hidden-file-input"
                      type="file"
                      accept="image/png,image/jpeg,image/webp,image/*"
                      onChange={(event) => void uploadImage(event)}
                    />
                    <button type="button" className="btn btn-sm" disabled={isImageMutating} onClick={() => imageInputRef.current?.click()}>
                      {isImageMutating ? 'Обновляем...' : 'Загрузить'}
                    </button>
                    <a className="btn btn-sm" href={buildPublicAssetImageUrl(asset.id, asset.updatedAt)} target="_blank" rel="noreferrer">
                      Открыть
                    </a>
                    <button type="button" className="btn btn-sm danger" disabled={isImageMutating} onClick={() => void deleteImage()}>
                      Удалить изображение
                    </button>
                  </div>
                </div>
              </div>

              <div className="admin-asset-edit-grid">
                <input
                  className="ui-input"
                  value={draft.displayName}
                  onChange={(event) => setDraft((prev) => prev ? { ...prev, displayName: event.target.value } : prev)}
                  placeholder="Название"
                />
                <textarea
                  className="ui-input"
                  value={draft.description}
                  onChange={(event) => setDraft((prev) => prev ? { ...prev, description: event.target.value } : prev)}
                  placeholder="Описание"
                  rows={1}
                />
                {asset.assetKind === 'skin' ? (
                  <>
                    <select
                      className="ui-input"
                      value={draft.rarity}
                      onChange={(event) => setDraft((prev) => prev ? { ...prev, rarity: event.target.value as SkinRarity | '' } : prev)}
                    >
                      <option value="">Без редкости</option>
                      <option value="common">common</option>
                      <option value="rare">rare</option>
                      <option value="legendary">legendary</option>
                    </select>
                    <input
                      className="ui-input"
                      value={draft.weaponKey}
                      onChange={(event) => setDraft((prev) => prev ? { ...prev, weaponKey: event.target.value } : prev)}
                      placeholder="weaponKey"
                    />
                  </>
                ) : (
                  <p className="admin-field-hint admin-shop-field--full">
                    Редкость и weaponKey доступны только для ассетов типа skin.
                  </p>
                )}
              </div>

              <div className="admin-asset-flags admin-asset-flags--radios">
                <fieldset className="ui-radio-group admin-asset-flag-fieldset">
                  <legend className="ui-radio-legend">Статус</legend>
                  <label className="ui-radio">
                    <input type="radio" name={`admin-asset-active-${asset.id}`} checked={draft.isActive} onChange={() => setDraft((prev) => prev ? { ...prev, isActive: true } : prev)} />
                    <span className="ui-radio-mark" aria-hidden />
                    <span>Активен</span>
                  </label>
                  <label className="ui-radio">
                    <input type="radio" name={`admin-asset-active-${asset.id}`} checked={!draft.isActive} onChange={() => setDraft((prev) => prev ? { ...prev, isActive: false } : prev)} />
                    <span className="ui-radio-mark" aria-hidden />
                    <span>Неактивен</span>
                  </label>
                </fieldset>
                <fieldset className="ui-radio-group admin-asset-flag-fieldset">
                  <legend className="ui-radio-legend">Видимость</legend>
                  <label className="ui-radio">
                    <input type="radio" name={`admin-asset-public-${asset.id}`} checked={draft.isPublic} onChange={() => setDraft((prev) => prev ? { ...prev, isPublic: true } : prev)} />
                    <span className="ui-radio-mark" aria-hidden />
                    <span>Публичный</span>
                  </label>
                  <label className="ui-radio">
                    <input type="radio" name={`admin-asset-public-${asset.id}`} checked={!draft.isPublic} onChange={() => setDraft((prev) => prev ? { ...prev, isPublic: false } : prev)} />
                    <span className="ui-radio-mark" aria-hidden />
                    <span>Скрытый</span>
                  </label>
                </fieldset>
                <fieldset className="ui-radio-group admin-asset-flag-fieldset">
                  <legend className="ui-radio-legend">Покупка</legend>
                  <label className="ui-radio">
                    <input type="radio" name={`admin-asset-purchasable-${asset.id}`} checked={draft.isUserPurchasable} onChange={() => setDraft((prev) => prev ? { ...prev, isUserPurchasable: true } : prev)} />
                    <span className="ui-radio-mark" aria-hidden />
                    <span>Можно покупать</span>
                  </label>
                  <label className="ui-radio">
                    <input type="radio" name={`admin-asset-purchasable-${asset.id}`} checked={!draft.isUserPurchasable} onChange={() => setDraft((prev) => prev ? { ...prev, isUserPurchasable: false } : prev)} />
                    <span className="ui-radio-mark" aria-hidden />
                    <span>Нельзя покупать</span>
                  </label>
                </fieldset>
              </div>

              <textarea
                className="ui-input admin-asset-metadata"
                value={draft.metadataText}
                onChange={(event) => setDraft((prev) => prev ? { ...prev, metadataText: event.target.value } : prev)}
                placeholder="Metadata JSON"
                rows={6}
              />
            </article>

            <div className="admin-shop-actions">
              <button type="button" className="btn btn-sm" disabled={isSaving} onClick={() => setDraft(makeEditDraft(asset))}>
                Отменить изменения
              </button>
              <button type="button" className="btn btn-sm primary" disabled={isSaving} onClick={() => void saveAsset()}>
                {isSaving ? 'Сохраняем...' : 'Сохранить'}
              </button>
            </div>
          </>
        ) : null}
      </section>
    </div>
  )
}
