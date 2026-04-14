import { useCallback, useEffect, useMemo, useRef, useState, type ChangeEvent } from 'react'
import toast from 'react-hot-toast'
import { toDisplayError } from '../../api/http'
import {
  buildPublicAssetImageUrl,
  createAdminAsset,
  deleteAdminAssetImage,
  listAdminAssets,
  patchAdminAsset,
  uploadAdminAssetImage,
  type AssetKind,
  type AssetResponse,
  type OwnershipModel,
  type SkinRarity,
} from '../../api/inventory'
import AppPortal from '../../shared/ui/portal/AppPortal'
import AdminAssetImage from './AdminAssetImage'
import ErrorState from '../ErrorState'
import LoadingState from '../LoadingState'

type AssetsState =
  | { status: 'loading' }
  | { status: 'error'; error: string }
  | { status: 'ready'; items: AssetResponse[]; total: number }

type CreateAssetDraft = {
  key: string
  displayName: string
  description: string
  assetKind: AssetKind
  ownershipModel: OwnershipModel
  rarity: SkinRarity | ''
  weaponKey: string
  isCurrency: boolean
  isUserPurchasable: boolean
  isPublic: boolean
  metadataText: string
}

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

const emptyCreateDraft: CreateAssetDraft = {
  key: '',
  displayName: '',
  description: '',
  assetKind: 'item',
  ownershipModel: 'stackable',
  rarity: '',
  weaponKey: '',
  isCurrency: false,
  isUserPurchasable: false,
  isPublic: true,
  metadataText: '{}',
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

function normalizeAssetKey(value: string) {
  return value.toLowerCase().replace(/[^a-z0-9_-]/g, '')
}

type AdminAssetSortKey = 'key_asc' | 'name_asc' | 'updated_desc' | 'updated_asc'

function sortAdminAssets(items: AssetResponse[], sort: AdminAssetSortKey): AssetResponse[] {
  const out = [...items]
  out.sort((a, b) => {
    switch (sort) {
      case 'key_asc':
        return a.key.localeCompare(b.key)
      case 'name_asc':
        return a.displayName.localeCompare(b.displayName, 'ru')
      case 'updated_desc':
        return b.updatedAt.localeCompare(a.updatedAt)
      case 'updated_asc':
        return a.updatedAt.localeCompare(b.updatedAt)
      default:
        return 0
    }
  })
  return out
}

export default function AdminAssetsPanel({ token }: { token: string }) {
  const [state, setState] = useState<AssetsState>({ status: 'loading' })
  const [query, setQuery] = useState('')
  const [draft, setDraft] = useState<CreateAssetDraft>(emptyCreateDraft)
  const [isCreating, setIsCreating] = useState(false)
  const [createModalOpen, setCreateModalOpen] = useState(false)
  const [catalogScope, setCatalogScope] = useState<'active_only' | 'all'>('active_only')
  const [assetSort, setAssetSort] = useState<AdminAssetSortKey>('key_asc')

  const loadAssets = useCallback(async () => {
    setState({ status: 'loading' })
    try {
      const response = await listAdminAssets(token, {
        q: query.trim() || undefined,
        page: 1,
        perPage: 100,
        ...(catalogScope === 'active_only' ? { isActive: true } : {}),
      })
      setState({ status: 'ready', items: response.items, total: response.total })
    } catch (cause) {
      setState({ status: 'error', error: toDisplayError(cause, 'Не удалось загрузить ассеты.') })
    }
  }, [catalogScope, query, token])

  useEffect(() => {
    queueMicrotask(() => void loadAssets())
  }, [loadAssets])

  const createAsset = async () => {
    if (isCreating) return

    const key = draft.key.trim()
    const displayName = draft.displayName.trim()
    if (!key || !displayName) {
      toast.error('Укажите key и название ассета.')
      return
    }

    let metadata: unknown
    try {
      metadata = parseMetadata(draft.metadataText)
    } catch {
      toast.error('Metadata должен быть валидным JSON.')
      return
    }

    setIsCreating(true)
    try {
      await createAdminAsset(token, {
        key,
        display_name: displayName,
        description: draft.description.trim() || null,
        asset_kind: draft.isCurrency ? 'currency' : draft.assetKind,
        ownership_model: draft.isCurrency ? 'stackable' : draft.ownershipModel,
        is_currency: draft.isCurrency,
        is_user_purchasable: draft.isUserPurchasable,
        is_public: draft.isPublic,
        rarity: draft.rarity || null,
        weaponKey: draft.weaponKey.trim() || null,
        metadata,
      })
      setDraft(emptyCreateDraft)
      setCreateModalOpen(false)
      await loadAssets()
      toast.success('Ассет создан.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось создать ассет.'))
    } finally {
      setIsCreating(false)
    }
  }

  const updateAsset = (asset: AssetResponse) => {
    setState((prev) => {
      if (prev.status !== 'ready') return prev
      return {
        ...prev,
        items: prev.items.map((item) => (item.id === asset.id ? asset : item)),
      }
    })
  }

  const mergeAsset = (updated: AssetResponse) => {
    if (catalogScope === 'active_only' && !updated.isActive) {
      void loadAssets()
      return
    }
    updateAsset(updated)
  }

  const sortedItems = useMemo(() => {
    if (state.status !== 'ready') return []
    return sortAdminAssets(state.items, assetSort)
  }, [assetSort, state])

  return (
    <section className="card admin-card">
      <div className="admin-assets-toolbar">
        <h2 className="card-title">Ассеты</h2>
        <div className="admin-assets-toolbar">
          <button type="button" className="btn btn-sm" onClick={() => void loadAssets()}>
            Обновить
          </button>
          <button type="button" className="btn primary btn-sm" onClick={() => setCreateModalOpen(true)}>
            Создать ассет…
          </button>
        </div>
      </div>

      <div className="admin-shop-list-controls">
        <div className="admin-shop-list-controls-row">
          <label className="admin-shop-field" style={{ marginBottom: 0 }}>
            <span>Сортировка</span>
            <select className="ui-input" value={assetSort} onChange={(e) => setAssetSort(e.target.value as AdminAssetSortKey)} aria-label="Сортировка списка ассетов">
              <option value="key_asc">Ключ (A–Я)</option>
              <option value="name_asc">Название (A–Я)</option>
              <option value="updated_desc">Обновление (сначала новые)</option>
              <option value="updated_asc">Обновление (сначала старые)</option>
            </select>
          </label>
          <fieldset className="ui-radio-group admin-asset-flag-fieldset">
            <legend className="ui-radio-legend">Состав каталога</legend>
            <label className="ui-radio">
              <input
                type="radio"
                name="admin-assets-catalog-scope"
                checked={catalogScope === 'active_only'}
                onChange={() => setCatalogScope('active_only')}
              />
              <span className="ui-radio-mark" aria-hidden />
              <span>Только активные</span>
            </label>
            <label className="ui-radio">
              <input
                type="radio"
                name="admin-assets-catalog-scope"
                checked={catalogScope === 'all'}
                onChange={() => setCatalogScope('all')}
              />
              <span className="ui-radio-mark" aria-hidden />
              <span>Все ассеты</span>
            </label>
          </fieldset>
        </div>
      </div>

      {createModalOpen ? (
        <AppPortal>
          <div
            className="ui-modal-backdrop"
            role="presentation"
            onClick={() => {
              if (!isCreating) setCreateModalOpen(false)
            }}
          >
            <div className="ui-modal admin-create-modal" role="dialog" aria-modal="true" aria-labelledby="admin-create-asset-title" onClick={(e) => e.stopPropagation()}>
              <div className="ui-modal-header">
                <h2 id="admin-create-asset-title" className="ui-modal-title">Новый ассет</h2>
                <button type="button" className="ui-modal-close" aria-label="Закрыть" disabled={isCreating} onClick={() => setCreateModalOpen(false)}>
                  ×
                </button>
              </div>
              <div className="ui-modal-body">
                <section className="admin-asset-create" aria-label="Создать ассет">
                  <div className="admin-asset-form-grid">
                    <input
                      className="ui-input"
                      value={draft.key}
                      onChange={(event) => setDraft((prev) => ({ ...prev, key: normalizeAssetKey(event.target.value) }))}
                      placeholder="key"
                    />
                    <input
                      className="ui-input"
                      value={draft.displayName}
                      onChange={(event) => setDraft((prev) => ({ ...prev, displayName: event.target.value }))}
                      placeholder="Название"
                    />
                    <input
                      className="ui-input"
                      value={draft.assetKind}
                      disabled={draft.isCurrency}
                      onChange={(event) => setDraft((prev) => ({ ...prev, assetKind: event.target.value }))}
                      placeholder="kind"
                    />
                    <select
                      className="ui-input"
                      value={draft.ownershipModel}
                      disabled={draft.isCurrency}
                      onChange={(event) => setDraft((prev) => ({ ...prev, ownershipModel: event.target.value as OwnershipModel }))}
                    >
                      <option value="stackable">stackable</option>
                      <option value="entitlement">entitlement</option>
                      <option value="expirable">expirable</option>
                    </select>
                    <select
                      className="ui-input"
                      value={draft.rarity}
                      onChange={(event) => setDraft((prev) => ({ ...prev, rarity: event.target.value as SkinRarity | '' }))}
                    >
                      <option value="">Без редкости</option>
                      <option value="common">common</option>
                      <option value="rare">rare</option>
                      <option value="legendary">legendary</option>
                    </select>
                    <input
                      className="ui-input"
                      value={draft.weaponKey}
                      onChange={(event) => setDraft((prev) => ({ ...prev, weaponKey: event.target.value }))}
                      placeholder="weaponKey"
                    />
                  </div>

                  <textarea
                    className="ui-input admin-asset-metadata"
                    value={draft.description}
                    onChange={(event) => setDraft((prev) => ({ ...prev, description: event.target.value }))}
                    placeholder="Описание"
                    rows={2}
                  />

                  <div className="admin-asset-flags admin-asset-flags--radios">
                    <fieldset className="ui-radio-group admin-asset-flag-fieldset">
                      <legend className="ui-radio-legend">Тип</legend>
                      <label className="ui-radio">
                        <input
                          type="radio"
                          name="admin-create-asset-currency"
                          checked={!draft.isCurrency}
                          onChange={() => setDraft((prev) => ({ ...prev, isCurrency: false }))}
                        />
                        <span className="ui-radio-mark" aria-hidden />
                        <span>Игровой ассет</span>
                      </label>
                      <label className="ui-radio">
                        <input
                          type="radio"
                          name="admin-create-asset-currency"
                          checked={draft.isCurrency}
                          onChange={() =>
                            setDraft((prev) => ({
                              ...prev,
                              isCurrency: true,
                              assetKind: 'currency',
                              ownershipModel: 'stackable',
                            }))}
                        />
                        <span className="ui-radio-mark" aria-hidden />
                        <span>Валюта</span>
                      </label>
                    </fieldset>
                    <fieldset className="ui-radio-group admin-asset-flag-fieldset">
                      <legend className="ui-radio-legend">Видимость</legend>
                      <label className="ui-radio">
                        <input
                          type="radio"
                          name="admin-create-asset-public"
                          checked={draft.isPublic}
                          onChange={() => setDraft((prev) => ({ ...prev, isPublic: true }))}
                        />
                        <span className="ui-radio-mark" aria-hidden />
                        <span>Публичный</span>
                      </label>
                      <label className="ui-radio">
                        <input
                          type="radio"
                          name="admin-create-asset-public"
                          checked={!draft.isPublic}
                          onChange={() => setDraft((prev) => ({ ...prev, isPublic: false }))}
                        />
                        <span className="ui-radio-mark" aria-hidden />
                        <span>Скрытый</span>
                      </label>
                    </fieldset>
                    <fieldset className="ui-radio-group admin-asset-flag-fieldset">
                      <legend className="ui-radio-legend">Покупка</legend>
                      <label className="ui-radio">
                        <input
                          type="radio"
                          name="admin-create-asset-purchasable"
                          checked={draft.isUserPurchasable}
                          onChange={() => setDraft((prev) => ({ ...prev, isUserPurchasable: true }))}
                        />
                        <span className="ui-radio-mark" aria-hidden />
                        <span>Можно покупать</span>
                      </label>
                      <label className="ui-radio">
                        <input
                          type="radio"
                          name="admin-create-asset-purchasable"
                          checked={!draft.isUserPurchasable}
                          onChange={() => setDraft((prev) => ({ ...prev, isUserPurchasable: false }))}
                        />
                        <span className="ui-radio-mark" aria-hidden />
                        <span>Нельзя покупать</span>
                      </label>
                    </fieldset>
                  </div>

                  <textarea
                    className="ui-input admin-asset-metadata"
                    value={draft.metadataText}
                    onChange={(event) => setDraft((prev) => ({ ...prev, metadataText: event.target.value }))}
                    placeholder="Metadata JSON"
                    rows={4}
                  />
                </section>
              </div>
              <div className="ui-modal-footer">
                <button type="button" className="btn btn-sm" disabled={isCreating} onClick={() => setCreateModalOpen(false)}>
                  Отмена
                </button>
                <button type="button" className="btn primary btn-sm" disabled={isCreating} onClick={() => void createAsset()}>
                  {isCreating ? 'Создаем...' : 'Создать ассет'}
                </button>
              </div>
            </div>
          </div>
        </AppPortal>
      ) : null}

      {state.status === 'loading' ? <LoadingState title="Загружаем ассеты" /> : null}
      {state.status === 'error' ? (
        <ErrorState title="Ассеты недоступны" message={state.error} primaryActionLabel="Повторить" onPrimaryAction={() => void loadAssets()} />
      ) : null}
      {state.status === 'ready' ? (
        <>
          <p className="admin-inline-muted">Найдено: {state.total}</p>
          <div className="admin-asset-list">
            {sortedItems.map((asset) => (
              <AdminAssetCard key={asset.id} token={token} asset={asset} onAssetChange={mergeAsset} />
            ))}
          </div>
        </>
      ) : null}
    </section>
  )
}

function AdminAssetCard({
  token,
  asset,
  onAssetChange,
}: {
  token: string
  asset: AssetResponse
  onAssetChange: (asset: AssetResponse) => void
}) {
  const [draft, setDraft] = useState(() => makeEditDraft(asset))
  const [isSaving, setIsSaving] = useState(false)
  const [isImageMutating, setIsImageMutating] = useState(false)
  const [imageReloadKey, setImageReloadKey] = useState(0)
  const imageInputRef = useRef<HTMLInputElement | null>(null)

  useEffect(() => {
    setDraft(makeEditDraft(asset))
  }, [asset])

  const uploadImage = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0] ?? null
    event.target.value = ''

    if (!file || isImageMutating) return
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
      onAssetChange(updated)
      setImageReloadKey((value) => value + 1)
      toast.success('Изображение ассета загружено.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось загрузить изображение ассета.'))
    } finally {
      setIsImageMutating(false)
    }
  }

  const deleteImage = async () => {
    if (isImageMutating) return

    setIsImageMutating(true)
    try {
      const updated = await deleteAdminAssetImage(token, asset.id)
      onAssetChange(updated)
      setImageReloadKey((value) => value + 1)
      toast.success('Изображение ассета удалено.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось удалить изображение ассета.'))
    } finally {
      setIsImageMutating(false)
    }
  }

  const saveAsset = async () => {
    if (isSaving) return
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
        rarity: draft.rarity || null,
        weaponKey: draft.weaponKey.trim() || null,
        metadata,
      })
      onAssetChange(updated)
      toast.success('Ассет обновлен.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось обновить ассет.'))
    } finally {
      setIsSaving(false)
    }
  }

  return (
    <article className="admin-asset-card">
      <div className="admin-asset-card-head">
        <div className="admin-row-user-text">
          <strong>{asset.displayName}</strong>
          <small>{asset.key} · {asset.id}</small>
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
              Удалить ассет
            </button>
          </div>
        </div>
      </div>

      <div className="admin-asset-edit-grid">
        <input
          className="ui-input"
          value={draft.displayName}
          onChange={(event) => setDraft((prev) => ({ ...prev, displayName: event.target.value }))}
          placeholder="Название"
        />
        <textarea
          className="ui-input"
          value={draft.description}
          onChange={(event) => setDraft((prev) => ({ ...prev, description: event.target.value }))}
          placeholder="Описание"
          rows={1}
        />
        <select
          className="ui-input"
          value={draft.rarity}
          onChange={(event) => setDraft((prev) => ({ ...prev, rarity: event.target.value as SkinRarity | '' }))}
        >
          <option value="">Без редкости</option>
          <option value="common">common</option>
          <option value="rare">rare</option>
          <option value="legendary">legendary</option>
        </select>
        <input
          className="ui-input"
          value={draft.weaponKey}
          onChange={(event) => setDraft((prev) => ({ ...prev, weaponKey: event.target.value }))}
          placeholder="weaponKey"
        />
      </div>

      <div className="admin-asset-flags admin-asset-flags--radios">
        <fieldset className="ui-radio-group admin-asset-flag-fieldset">
          <legend className="ui-radio-legend">Статус</legend>
          <label className="ui-radio">
            <input
              type="radio"
              name={`admin-asset-active-${asset.id}`}
              checked={draft.isActive}
              onChange={() => setDraft((prev) => ({ ...prev, isActive: true }))}
            />
            <span className="ui-radio-mark" aria-hidden />
            <span>Активен</span>
          </label>
          <label className="ui-radio">
            <input
              type="radio"
              name={`admin-asset-active-${asset.id}`}
              checked={!draft.isActive}
              onChange={() => setDraft((prev) => ({ ...prev, isActive: false }))}
            />
            <span className="ui-radio-mark" aria-hidden />
            <span>Неактивен</span>
          </label>
        </fieldset>
        <fieldset className="ui-radio-group admin-asset-flag-fieldset">
          <legend className="ui-radio-legend">Видимость</legend>
          <label className="ui-radio">
            <input
              type="radio"
              name={`admin-asset-public-${asset.id}`}
              checked={draft.isPublic}
              onChange={() => setDraft((prev) => ({ ...prev, isPublic: true }))}
            />
            <span className="ui-radio-mark" aria-hidden />
            <span>Публичный</span>
          </label>
          <label className="ui-radio">
            <input
              type="radio"
              name={`admin-asset-public-${asset.id}`}
              checked={!draft.isPublic}
              onChange={() => setDraft((prev) => ({ ...prev, isPublic: false }))}
            />
            <span className="ui-radio-mark" aria-hidden />
            <span>Скрытый</span>
          </label>
        </fieldset>
        <fieldset className="ui-radio-group admin-asset-flag-fieldset">
          <legend className="ui-radio-legend">Покупка</legend>
          <label className="ui-radio">
            <input
              type="radio"
              name={`admin-asset-purchasable-${asset.id}`}
              checked={draft.isUserPurchasable}
              onChange={() => setDraft((prev) => ({ ...prev, isUserPurchasable: true }))}
            />
            <span className="ui-radio-mark" aria-hidden />
            <span>Можно покупать</span>
          </label>
          <label className="ui-radio">
            <input
              type="radio"
              name={`admin-asset-purchasable-${asset.id}`}
              checked={!draft.isUserPurchasable}
              onChange={() => setDraft((prev) => ({ ...prev, isUserPurchasable: false }))}
            />
            <span className="ui-radio-mark" aria-hidden />
            <span>Нельзя покупать</span>
          </label>
        </fieldset>
      </div>

      <textarea
        className="ui-input admin-asset-metadata"
        value={draft.metadataText}
        onChange={(event) => setDraft((prev) => ({ ...prev, metadataText: event.target.value }))}
        placeholder="Metadata JSON"
        rows={4}
      />

      <div className="admin-asset-actions">
        <button type="button" className="btn btn-sm" disabled={isSaving} onClick={() => setDraft(makeEditDraft(asset))}>
          Отменить
        </button>
        <button type="button" className="btn btn-sm primary" disabled={isSaving} onClick={() => void saveAsset()}>
          {isSaving ? 'Сохраняем...' : 'Сохранить'}
        </button>
      </div>
    </article>
  )
}
