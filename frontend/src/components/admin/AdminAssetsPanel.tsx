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
import { adminAssetPath } from '../../routes/paths'
import AppPortal from '../../shared/ui/portal/AppPortal'
import AdminAssetImage from './AdminAssetImage'
import AdminLink from './AdminLink'
import ErrorState from '../ErrorState'
import LoadingState from '../LoadingState'

type AssetsState =
  | { status: 'loading' }
  | { status: 'error'; error: string }
  | { status: 'ready'; items: AssetResponse[]; total: number; page: number; perPage: number; totalPages: number }

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

const assetKindOptions: AssetKind[] = ['item', 'skin', 'subscription', 'cosmetic', 'lootbox', 'currency', 'ticket', 'token', 'kit']

function normalizeCreateDraftForType(draft: CreateAssetDraft): CreateAssetDraft {
  if (draft.isCurrency) {
    return {
      ...draft,
      assetKind: 'currency',
      ownershipModel: 'stackable',
      rarity: '',
      weaponKey: '',
    }
  }

  if (draft.assetKind === 'kit') {
    return {
      ...draft,
      ownershipModel: 'stackable',
      rarity: '',
      weaponKey: '',
    }
  }

  if (draft.assetKind !== 'skin') {
    return {
      ...draft,
      rarity: '',
      weaponKey: '',
    }
  }

  return draft
}

function parseMetadata(value: string) {
  const trimmed = value.trim()
  if (!trimmed) return null
  return JSON.parse(trimmed) as unknown
}

function metadataToText(value: unknown) {
  if (value == null) return '{}'
  try {
    return JSON.stringify(value, null, 2)
  } catch {
    return String(value)
  }
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
const ASSETS_PER_PAGE = 20

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
  const [draft, setDraft] = useState<CreateAssetDraft>(emptyCreateDraft)
  const [isCreating, setIsCreating] = useState(false)
  const [createModalOpen, setCreateModalOpen] = useState(false)
  const [catalogScope, setCatalogScope] = useState<'active_only' | 'all'>('active_only')
  const [assetSort, setAssetSort] = useState<AdminAssetSortKey>('key_asc')
  const [assetSearch, setAssetSearch] = useState('')
  const [page, setPage] = useState(1)

  const loadAssets = useCallback(async () => {
    setState({ status: 'loading' })
    try {
      const response = await listAdminAssets(token, {
        page,
        perPage: ASSETS_PER_PAGE,
        q: assetSearch.trim() || undefined,
        ...(catalogScope === 'active_only' ? { isActive: true } : {}),
      })
      setState({
        status: 'ready',
        items: response.items,
        total: response.total,
        page: response.page,
        perPage: response.perPage,
        totalPages: response.totalPages,
      })
    } catch (cause) {
      setState({ status: 'error', error: toDisplayError(cause, 'Не удалось загрузить ассеты.') })
    }
  }, [assetSearch, catalogScope, page, token])

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
      const normalizedDraft = normalizeCreateDraftForType(draft)
      const isSkin = normalizedDraft.assetKind === 'skin'
      await createAdminAsset(token, {
        key,
        display_name: displayName,
        description: normalizedDraft.description.trim() || null,
        asset_kind: normalizedDraft.assetKind,
        ownership_model: normalizedDraft.ownershipModel,
        is_currency: normalizedDraft.isCurrency,
        is_user_purchasable: normalizedDraft.isUserPurchasable,
        is_public: normalizedDraft.isPublic,
        rarity: isSkin ? normalizedDraft.rarity || null : null,
        weaponKey: isSkin ? normalizedDraft.weaponKey.trim() || null : null,
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

  const sortedItems = useMemo(() => {
    if (state.status !== 'ready') return []
    return sortAdminAssets(state.items, assetSort)
  }, [assetSort, state])

  const setCatalogScopeAndReset = (value: 'active_only' | 'all') => {
    setCatalogScope(value)
    setPage(1)
  }

  const setSearchAndReset = (value: string) => {
    setAssetSearch(value)
    setPage(1)
  }

  const totalPages = state.status === 'ready' ? Math.max(1, state.totalPages) : 1
  const activeAssets = state.status === 'ready' ? state.items.filter((item) => item.isActive).length : 0
  const hiddenAssets = state.status === 'ready' ? state.items.filter((item) => !item.isPublic).length : 0

  return (
    <section className="card admin-card">
      <div className="admin-section-head">
        <div>
          <h2 className="card-title">Ассеты</h2>
          <p className="card-text">Каталог предметов, валют, скинов и наград для магазина и лутбоксов.</p>
        </div>
        <div className="admin-assets-toolbar">
          <button type="button" className="btn btn-sm" onClick={() => void loadAssets()}>
            Обновить
          </button>
          <button type="button" className="btn primary btn-sm" onClick={() => setCreateModalOpen(true)}>
            Создать ассет…
          </button>
        </div>
      </div>

      {state.status === 'ready' ? (
        <div className="admin-metric-strip">
          <div className="admin-metric">
            <span>Найдено</span>
            <strong>{state.total}</strong>
          </div>
          <div className="admin-metric admin-metric--success">
            <span>Активны на странице</span>
            <strong>{activeAssets}</strong>
          </div>
          <div className="admin-metric admin-metric--warning">
            <span>Скрыты</span>
            <strong>{hiddenAssets}</strong>
          </div>
        </div>
      ) : null}

      <div className="admin-shop-list-controls admin-filter-bar">
        <div className="admin-shop-list-controls-row">
          <label className="admin-shop-field admin-filter-label">
            <span>Поиск</span>
            <input
              className="ui-input"
              value={assetSearch}
              onChange={(event) => setSearchAndReset(event.target.value)}
              placeholder="Название или key"
            />
          </label>
          <label className="admin-shop-field admin-filter-label">
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
                onChange={() => setCatalogScopeAndReset('active_only')}
              />
              <span className="ui-radio-mark" aria-hidden />
              <span>Только активные</span>
            </label>
            <label className="ui-radio">
              <input
                type="radio"
                name="admin-assets-catalog-scope"
                checked={catalogScope === 'all'}
                onChange={() => setCatalogScopeAndReset('all')}
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
                      aria-label="Ключ ассета"
                    />
                    <input
                      className="ui-input"
                      value={draft.displayName}
                      onChange={(event) => setDraft((prev) => ({ ...prev, displayName: event.target.value }))}
                      placeholder="Название"
                      aria-label="Название ассета"
                    />
                    <select
                      className="ui-input"
                      value={draft.assetKind}
                      disabled={draft.isCurrency}
                      onChange={(event) =>
                        setDraft((prev) =>
                          normalizeCreateDraftForType({
                            ...prev,
                            assetKind: event.target.value as AssetKind,
                          }),
                        )}
                      aria-label="Тип ассета"
                    >
                      {assetKindOptions.map((kind) => (
                        <option key={kind} value={kind}>{kind}</option>
                      ))}
                    </select>
                    <select
                      className="ui-input"
                      value={draft.ownershipModel}
                      disabled={draft.isCurrency || draft.assetKind === 'kit'}
                      onChange={(event) => setDraft((prev) => ({ ...prev, ownershipModel: event.target.value as OwnershipModel }))}
                      aria-label="Модель владения"
                    >
                      <option value="stackable">stackable</option>
                      <option value="entitlement">entitlement</option>
                      <option value="expirable">expirable</option>
                    </select>
                    {draft.assetKind === 'skin' ? (
                      <>
                        <select
                          className="ui-input"
                          value={draft.rarity}
                          onChange={(event) => setDraft((prev) => ({ ...prev, rarity: event.target.value as SkinRarity | '' }))}
                          aria-label="Редкость"
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
                          aria-label="Ключ оружия"
                        />
                      </>
                    ) : (
                      <p className="admin-field-hint admin-shop-field--full">
                        Редкость и weaponKey доступны только для ассетов типа skin.
                      </p>
                    )}
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
                          onChange={() => setDraft((prev) => normalizeCreateDraftForType({ ...prev, isCurrency: false, assetKind: 'item' }))}
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
                              rarity: '',
                              weaponKey: '',
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
          <div className="admin-assets-result-head">
            <p className="admin-inline-muted">
              Найдено: {state.total}. Страница {state.page} из {totalPages}
            </p>
          </div>

          <div className="admin-asset-table-wrap">
            <div className="admin-asset-table admin-asset-list" aria-label="Ассеты">
              <div className="admin-asset-list-head">
                <span>Ассет</span>
                <span>Тип</span>
                <span>Модель</span>
                <span>Флаги</span>
                <span>Обновлен</span>
              </div>
              <div className="admin-asset-list-body">
                {sortedItems.map((asset) => (
                  <AdminLink
                    key={asset.id}
                    className="admin-asset-list-row admin-asset-table-row"
                    href={adminAssetPath(asset.id)}
                    aria-label={`Открыть ассет ${asset.displayName}`}
                  >
                    <span className="admin-asset-list-cell admin-asset-list-cell--asset">
                      <span className="admin-inventory-item-main admin-asset-table-link">
                        <AdminAssetImage token={token} asset={asset} className="admin-asset-image-preview--thumb" />
                        <span className="admin-row-inventory-text">
                          <strong>{asset.displayName}</strong>
                          <small>{asset.key} · {asset.id}</small>
                        </span>
                      </span>
                    </span>
                    <span className="admin-asset-list-cell">
                      <span className="ui-badge ui-badge-neutral">{asset.assetKind}</span>
                    </span>
                    <span className="admin-asset-list-cell">
                      <span className="ui-badge ui-badge-neutral">{asset.ownershipModel}</span>
                    </span>
                    <span className="admin-asset-list-cell">
                      <span className="admin-asset-badges">
                        {asset.isCurrency ? <span className="ui-badge ui-badge-secondary">currency</span> : null}
                        {asset.isPublic ? <span className="ui-badge ui-badge-neutral">public</span> : <span className="ui-badge ui-badge-warning">hidden</span>}
                        {asset.isUserPurchasable ? <span className="ui-badge ui-badge-neutral">purchasable</span> : null}
                        <span className={`ui-badge ${asset.isActive ? 'ui-badge-success' : 'ui-badge-warning'}`}>
                          {asset.isActive ? 'active' : 'inactive'}
                        </span>
                      </span>
                    </span>
                    <span className="admin-asset-list-cell">{new Date(asset.updatedAt).toLocaleString('ru-RU')}</span>
                  </AdminLink>
                ))}
                {!sortedItems.length ? (
                  <p className="admin-asset-table-empty">Ассеты не найдены.</p>
                ) : null}
              </div>
            </div>
          </div>

          <AdminPagination page={page} totalPages={totalPages} onPageChange={setPage} />
        </>
      ) : null}
    </section>
  )
}

function getPaginationPages(page: number, totalPages: number) {
  const pages = new Set([1, totalPages, page - 1, page, page + 1].filter((item) => item >= 1 && item <= totalPages))
  return Array.from(pages).sort((left, right) => left - right)
}

function AdminPagination({
  page,
  totalPages,
  onPageChange,
}: {
  page: number
  totalPages: number
  onPageChange: (page: number) => void
}) {
  const pages = getPaginationPages(page, totalPages)

  return (
    <nav className="admin-pagination" aria-label="Нумерация страниц ассетов">
      <ul className="ui-pagination">
        <li>
          <button type="button" className="ui-pagination-btn" aria-label="Предыдущая страница" disabled={page <= 1} onClick={() => onPageChange(Math.max(1, page - 1))}>‹</button>
        </li>
        {pages.map((item, index) => (
          <li key={item}>
            {index > 0 && item - pages[index - 1] > 1 ? <span className="ui-pagination-ellipsis">…</span> : null}
            <button
              type="button"
              className="ui-pagination-btn"
              aria-label={`Страница ${item}`}
              aria-current={page === item ? 'page' : undefined}
              onClick={() => onPageChange(item)}
            >
              {item}
            </button>
          </li>
        ))}
        <li>
          <button type="button" className="ui-pagination-btn" aria-label="Следующая страница" disabled={page >= totalPages} onClick={() => onPageChange(Math.min(totalPages, page + 1))}>›</button>
        </li>
      </ul>
    </nav>
  )
}

export function AdminAssetModal({
  token,
  asset,
  onClose,
  onAssetChange,
}: {
  token: string
  asset: AssetResponse
  onClose: () => void
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
        ...(asset.assetKind === 'skin'
          ? {
              rarity: draft.rarity || null,
              weaponKey: draft.weaponKey.trim() || null,
            }
          : {}),
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
    <AppPortal>
      <div className="ui-modal-backdrop" role="presentation" onClick={() => !isSaving && !isImageMutating && onClose()}>
        <div className="ui-modal admin-asset-edit-modal" role="dialog" aria-modal="true" aria-labelledby="admin-edit-asset-title" onClick={(event) => event.stopPropagation()}>
          <div className="ui-modal-header">
            <h2 id="admin-edit-asset-title" className="ui-modal-title">Редактировать ассет</h2>
            <button type="button" className="ui-modal-close" aria-label="Закрыть" disabled={isSaving || isImageMutating} onClick={onClose}>
              ×
            </button>
          </div>
          <div className="ui-modal-body admin-asset-edit-modal-body">
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
                      Удалить изображение
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
                {asset.assetKind === 'skin' ? (
                  <>
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
            </article>
          </div>
          <div className="ui-modal-footer">
            <button type="button" className="btn btn-sm" disabled={isSaving} onClick={() => setDraft(makeEditDraft(asset))}>
              Отменить изменения
            </button>
            <button type="button" className="btn btn-sm primary" disabled={isSaving} onClick={() => void saveAsset()}>
              {isSaving ? 'Сохраняем...' : 'Сохранить'}
            </button>
          </div>
        </div>
      </div>
    </AppPortal>
  )
}
