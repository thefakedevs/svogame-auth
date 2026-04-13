import { useCallback, useEffect, useState } from 'react'
import toast from 'react-hot-toast'
import { toDisplayError } from '../../api/http'
import {
  createAdminAsset,
  listAdminAssets,
  patchAdminAsset,
  type AssetKind,
  type AssetResponse,
  type OwnershipModel,
} from '../../api/ownership'
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
  isCurrency: boolean
  isUserPurchasable: boolean
  isPublic: boolean
  metadataText: string
}

type EditAssetDraft = {
  displayName: string
  description: string
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
    isActive: asset.isActive,
    isPublic: asset.isPublic,
    isUserPurchasable: asset.isUserPurchasable,
    metadataText: metadataToText(asset.metadata),
  }
}

function normalizeAssetKey(value: string) {
  return value.toLowerCase().replace(/[^a-z0-9_-]/g, '')
}

export default function AdminAssetsPanel({ token }: { token: string }) {
  const [state, setState] = useState<AssetsState>({ status: 'loading' })
  const [query, setQuery] = useState('')
  const [draft, setDraft] = useState<CreateAssetDraft>(emptyCreateDraft)
  const [isCreating, setIsCreating] = useState(false)

  const loadAssets = useCallback(async () => {
    setState({ status: 'loading' })
    try {
      const response = await listAdminAssets(token, {
        q: query.trim() || undefined,
        page: 1,
        perPage: 100,
      })
      setState({ status: 'ready', items: response.items, total: response.total })
    } catch (cause) {
      setState({ status: 'error', error: toDisplayError(cause, 'Не удалось загрузить ассеты.') })
    }
  }, [query, token])

  useEffect(() => {
    queueMicrotask(() => void loadAssets())
  }, [loadAssets])

  const updateCreatedAsset = (asset: AssetResponse) => {
    setState((prev) => {
      if (prev.status !== 'ready') return prev
      const exists = prev.items.some((item) => item.id === asset.id)
      return {
        status: 'ready',
        total: exists ? prev.total : prev.total + 1,
        items: exists
          ? prev.items.map((item) => (item.id === asset.id ? asset : item))
          : [asset, ...prev.items],
      }
    })
  }

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
      const created = await createAdminAsset(token, {
        key,
        display_name: displayName,
        description: draft.description.trim() || null,
        asset_kind: draft.isCurrency ? 'currency' : draft.assetKind,
        ownership_model: draft.isCurrency ? 'stackable' : draft.ownershipModel,
        is_currency: draft.isCurrency,
        is_user_purchasable: draft.isUserPurchasable,
        is_public: draft.isPublic,
        metadata,
      })
      updateCreatedAsset(created)
      setDraft(emptyCreateDraft)
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

  return (
    <section className="card admin-card">
      <div className="admin-assets-toolbar">
        <h2 className="card-title">Ассеты</h2>
        <div className="admin-assets-search">
          <input
            className="ui-input"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Поиск по key или названию"
          />
          <button type="button" className="btn btn-sm" onClick={() => void loadAssets()}>
            Обновить
          </button>
        </div>
      </div>

      <section className="admin-asset-create" aria-label="Создать ассет">
        <h3 className="card-title">Новый ассет</h3>
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
        </div>

        <textarea
          className="ui-input admin-asset-metadata"
          value={draft.description}
          onChange={(event) => setDraft((prev) => ({ ...prev, description: event.target.value }))}
          placeholder="Описание"
          rows={2}
        />

        <div className="admin-asset-flags">
          <label className="admin-checkbox-row">
            <input
              type="checkbox"
              checked={draft.isCurrency}
              onChange={(event) => setDraft((prev) => ({
                ...prev,
                isCurrency: event.target.checked,
                assetKind: event.target.checked ? 'currency' : prev.assetKind,
                ownershipModel: event.target.checked ? 'stackable' : prev.ownershipModel,
              }))}
            />
            Валюта
          </label>
          <label className="admin-checkbox-row">
            <input
              type="checkbox"
              checked={draft.isPublic}
              onChange={(event) => setDraft((prev) => ({ ...prev, isPublic: event.target.checked }))}
            />
            Публичный
          </label>
          <label className="admin-checkbox-row">
            <input
              type="checkbox"
              checked={draft.isUserPurchasable}
              onChange={(event) => setDraft((prev) => ({ ...prev, isUserPurchasable: event.target.checked }))}
            />
            Можно покупать
          </label>
        </div>

        <textarea
          className="ui-input admin-asset-metadata"
          value={draft.metadataText}
          onChange={(event) => setDraft((prev) => ({ ...prev, metadataText: event.target.value }))}
          placeholder="Metadata JSON"
          rows={4}
        />

        <button type="button" className="btn primary" disabled={isCreating} onClick={() => void createAsset()}>
          {isCreating ? 'Создаем...' : 'Создать ассет'}
        </button>
      </section>

      {state.status === 'loading' ? <LoadingState title="Загружаем ассеты" /> : null}
      {state.status === 'error' ? (
        <ErrorState title="Ассеты недоступны" message={state.error} primaryActionLabel="Повторить" onPrimaryAction={() => void loadAssets()} />
      ) : null}
      {state.status === 'ready' ? (
        <>
          <p className="admin-inline-muted">Найдено: {state.total}</p>
          <div className="admin-asset-list">
            {state.items.map((asset) => (
              <AdminAssetCard key={asset.id} token={token} asset={asset} onAssetChange={updateAsset} />
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

  useEffect(() => {
    setDraft(makeEditDraft(asset))
  }, [asset])

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

      <div className="admin-asset-edit-grid">
        <input
          className="ui-input"
          value={draft.displayName}
          onChange={(event) => setDraft((prev) => ({ ...prev, displayName: event.target.value }))}
          placeholder="Название"
        />
        <input
          className="ui-input"
          value={draft.description}
          onChange={(event) => setDraft((prev) => ({ ...prev, description: event.target.value }))}
          placeholder="Описание"
        />
      </div>

      <div className="admin-asset-flags">
        <label className="admin-checkbox-row">
          <input
            type="checkbox"
            checked={draft.isActive}
            onChange={(event) => setDraft((prev) => ({ ...prev, isActive: event.target.checked }))}
          />
          Активен
        </label>
        <label className="admin-checkbox-row">
          <input
            type="checkbox"
            checked={draft.isPublic}
            onChange={(event) => setDraft((prev) => ({ ...prev, isPublic: event.target.checked }))}
          />
          Публичный
        </label>
        <label className="admin-checkbox-row">
          <input
            type="checkbox"
            checked={draft.isUserPurchasable}
            onChange={(event) => setDraft((prev) => ({ ...prev, isUserPurchasable: event.target.checked }))}
          />
          Можно покупать
        </label>
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
