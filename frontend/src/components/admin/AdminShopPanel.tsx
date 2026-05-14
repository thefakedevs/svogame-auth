import { useCallback, useEffect, useMemo, useState } from 'react'
import toast from 'react-hot-toast'
import { createAdminShopProduct, listAdminShopProducts, type CreateShopProductInput } from '../../api/admin'
import { toDisplayError } from '../../api/http'
import { listAllAdminAssets, type AssetResponse, type OwnershipModel } from '../../api/inventory'
import type { ShopProductResponse } from '../../api/shop'
import { adminShopProductPath } from '../../routes/paths'
import AppPortal from '../../shared/ui/portal/AppPortal'
import ErrorState from '../ErrorState'
import LoadingState from '../LoadingState'
import AdminLink from './AdminLink'

type LocaleRow = { locale: string; name: string; description: string }

const defaultLocaleRows: LocaleRow[] = [{ locale: 'ru-RU', name: '', description: '' }]
const SHOP_PRODUCTS_PER_PAGE = 20

type ShopAdminSortKey = 'catalog' | 'key' | 'price_asc' | 'price_desc' | 'name' | 'updated_desc'

function sortAdminShopProducts(list: ShopProductResponse[], sort: ShopAdminSortKey): ShopProductResponse[] {
  const out = [...list]
  const byKey = (a: ShopProductResponse, b: ShopProductResponse) => a.key.localeCompare(b.key)
  switch (sort) {
    case 'catalog':
      out.sort((a, b) => a.sortOrder - b.sortOrder || byKey(a, b))
      break
    case 'key':
      out.sort(byKey)
      break
    case 'price_asc':
      out.sort((a, b) => a.priceRub - b.priceRub || byKey(a, b))
      break
    case 'price_desc':
      out.sort((a, b) => b.priceRub - a.priceRub || byKey(a, b))
      break
    case 'name':
      out.sort((a, b) => a.localizedName.localeCompare(b.localizedName, 'ru') || byKey(a, b))
      break
    case 'updated_desc':
      out.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt) || byKey(a, b))
      break
    default:
      break
  }
  return out
}

function getPaginationPages(page: number, totalPages: number) {
  const pages = new Set([1, totalPages, page - 1, page, page + 1].filter((item) => item >= 1 && item <= totalPages))
  return Array.from(pages).sort((left, right) => left - right)
}

function AdminPagination({
  page,
  totalPages,
  onPageChange,
  label,
}: {
  page: number
  totalPages: number
  onPageChange: (page: number) => void
  label: string
}) {
  const pages = getPaginationPages(page, totalPages)

  return (
    <nav className="admin-pagination" aria-label={label}>
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

function buildCreatePayload(
  key: string,
  assetKey: string,
  ownershipModel: OwnershipModel,
  priceRub: number,
  localeRows: LocaleRow[],
  extras: {
    stackableAmount: string
    durationSeconds: string
    maxPerPurchase: string
    maxOwnedAmount: string
    sortOrder: string
    startsAt: string
    endsAt: string
    isActive: boolean
    isPublic: boolean
    metadataText: string
  },
): CreateShopProductInput {
  const locales = localeRows
    .map((row) => ({
      locale: row.locale.trim(),
      name: row.name.trim(),
      description: row.description.trim() || null,
    }))
    .filter((row) => row.locale && row.name)

  const body: CreateShopProductInput = {
    key: key.trim(),
    asset_key: assetKey.trim(),
    price_rub: priceRub,
    locales,
  }

  const stack = extras.stackableAmount.trim()
  const dur = extras.durationSeconds.trim()
  const mpp = extras.maxPerPurchase.trim()
  const moa = extras.maxOwnedAmount.trim()

  if (ownershipModel === 'stackable') {
    body.stackable_amount = Number(stack)
    if (mpp) body.max_per_purchase = Number(mpp)
    if (moa) body.max_owned_amount = Number(moa)
  }

  if (ownershipModel === 'expirable') {
    body.durationSeconds = Number(dur)
  }

  const so = extras.sortOrder.trim()
  if (so) body.sort_order = Number(so)

  const sa = extras.startsAt.trim()
  if (sa) body.starts_at = sa

  const ea = extras.endsAt.trim()
  if (ea) body.ends_at = ea

  body.is_active = extras.isActive
  body.is_public = extras.isPublic

  const metaTrim = extras.metadataText.trim()
  if (metaTrim) body.metadata = JSON.parse(metaTrim) as unknown

  return body
}

export default function AdminShopPanel({ token }: { token: string }) {
  const [items, setItems] = useState<ShopProductResponse[] | null>(null)
  const [listError, setListError] = useState('')

  const [catalogAssets, setCatalogAssets] = useState<AssetResponse[] | null>(null)
  const [assetsError, setAssetsError] = useState('')

  const [createModalOpen, setCreateModalOpen] = useState(false)
  const [productCatalogScope, setProductCatalogScope] = useState<'active_only' | 'all'>('active_only')
  const [shopSort, setShopSort] = useState<ShopAdminSortKey>('catalog')
  const [shopQuery, setShopQuery] = useState('')
  const [shopPage, setShopPage] = useState(1)

  const [productKey, setProductKey] = useState('')
  const [assetKey, setAssetKey] = useState('')
  const [priceRub, setPriceRub] = useState('')
  const [localeRows, setLocaleRows] = useState<LocaleRow[]>(defaultLocaleRows)
  const [stackableAmount, setStackableAmount] = useState('')
  const [durationSeconds, setDurationSeconds] = useState('')
  const [maxPerPurchase, setMaxPerPurchase] = useState('')
  const [maxOwnedAmount, setMaxOwnedAmount] = useState('')
  const [sortOrder, setSortOrder] = useState('')
  const [startsAt, setStartsAt] = useState('')
  const [endsAt, setEndsAt] = useState('')
  const [isActive, setIsActive] = useState(true)
  const [isPublic, setIsPublic] = useState(true)
  const [metadataText, setMetadataText] = useState('{}')
  const [isCreating, setIsCreating] = useState(false)

  const loadProducts = useCallback(
    async (localeFilter: string) => {
      setListError('')
      setItems(null)
      try {
        const list = await listAdminShopProducts(token, localeFilter.trim() || undefined)
        setItems(list)
      } catch (cause) {
        setListError(toDisplayError(cause, 'Не удалось загрузить товары магазина.'))
      }
    },
    [token],
  )

  useEffect(() => {
    void loadProducts('')
  }, [loadProducts])

  useEffect(() => {
    let cancelled = false
    const run = async () => {
      setAssetsError('')
      setCatalogAssets(null)
      try {
        const all = await listAllAdminAssets(token)
        if (!cancelled) setCatalogAssets(all)
      } catch (cause) {
        if (!cancelled) setAssetsError(toDisplayError(cause, 'Не удалось загрузить ассеты для выбора.'))
      }
    }
    void run()
    return () => {
      cancelled = true
    }
  }, [token])

  const usedShopAssetKeys = useMemo(() => new Set((items ?? []).map((p) => p.assetKey)), [items])

  const pickableAssets = useMemo(() => {
    if (!catalogAssets) return []
    return catalogAssets.filter((a) => !a.isCurrency && !usedShopAssetKeys.has(a.key))
  }, [catalogAssets, usedShopAssetKeys])

  const selectedAsset = useMemo(
    () => pickableAssets.find((asset) => asset.key === assetKey) ?? null,
    [assetKey, pickableAssets],
  )

  useEffect(() => {
    if (!assetKey || !catalogAssets) return
    const allowed = new Set(pickableAssets.map((a) => a.key))
    if (!allowed.has(assetKey)) setAssetKey('')
  }, [assetKey, catalogAssets, pickableAssets])

  const displayedProducts = useMemo(() => {
    if (!items) return []
    const needle = shopQuery.trim().toLowerCase()
    const scoped = productCatalogScope === 'active_only' ? items.filter((p) => p.isActive) : items
    const filtered = needle
      ? scoped.filter((p) =>
          `${p.key} ${p.assetKey} ${p.localizedName} ${p.localizedDescription ?? ''}`.toLowerCase().includes(needle),
        )
      : scoped
    return sortAdminShopProducts(filtered, shopSort)
  }, [items, productCatalogScope, shopQuery, shopSort])

  const totalPages = Math.max(1, Math.ceil(displayedProducts.length / SHOP_PRODUCTS_PER_PAGE))
  const pagedProducts = displayedProducts.slice((shopPage - 1) * SHOP_PRODUCTS_PER_PAGE, shopPage * SHOP_PRODUCTS_PER_PAGE)

  useEffect(() => {
    setShopPage((current) => Math.min(current, totalPages))
  }, [totalPages])

  const setShopQueryAndReset = (value: string) => {
    setShopQuery(value)
    setShopPage(1)
  }

  const setProductCatalogScopeAndReset = (value: 'active_only' | 'all') => {
    setProductCatalogScope(value)
    setShopPage(1)
  }

  const updateLocaleRow = (index: number, patch: Partial<LocaleRow>) => {
    setLocaleRows((prev) => prev.map((row, i) => (i === index ? { ...row, ...patch } : row)))
  }

  const addLocaleRow = () => {
    setLocaleRows((prev) => [...prev, { locale: 'en-US', name: '', description: '' }])
  }

  const removeLocaleRow = (index: number) => {
    setLocaleRows((prev) => (prev.length <= 1 ? prev : prev.filter((_, i) => i !== index)))
  }

  const createProduct = async () => {
    const price = Number(priceRub)
    if (!productKey.trim() || !assetKey.trim()) {
      toast.error('Укажите ключ товара и выберите ассет.')
      return
    }
    if (!Number.isFinite(price) || price < 0) {
      toast.error('Укажите корректную цену в рублях.')
      return
    }
    if (!selectedAsset) {
      toast.error('Выберите ассет для товара.')
      return
    }
    if (selectedAsset.ownershipModel === 'stackable') {
      const amount = Number(stackableAmount)
      if (!Number.isFinite(amount) || amount <= 0) {
        toast.error('Для stackable-товара укажите положительный stackable_amount.')
        return
      }
    }
    if (selectedAsset.ownershipModel === 'expirable') {
      const duration = Number(durationSeconds)
      if (!Number.isFinite(duration) || duration <= 0) {
        toast.error('Для expirable-товара укажите положительный durationSeconds.')
        return
      }
    }
    let payload: CreateShopProductInput
    try {
      payload = buildCreatePayload(productKey, assetKey, selectedAsset.ownershipModel, price, localeRows, {
        stackableAmount,
        durationSeconds,
        maxPerPurchase,
        maxOwnedAmount,
        sortOrder,
        startsAt,
        endsAt,
        isActive,
        isPublic,
        metadataText,
      })
    } catch {
      toast.error('Некорректный JSON в поле metadata.')
      return
    }
    if (!payload.locales.length) {
      toast.error('Добавьте хотя бы одну локаль с кодом и названием.')
      return
    }
    setIsCreating(true)
    try {
      await createAdminShopProduct(token, payload)
      await loadProducts('')
      toast.success('Товар создан.')
      setProductKey('')
      setAssetKey('')
      setPriceRub('')
      setLocaleRows(defaultLocaleRows)
      setStackableAmount('')
      setDurationSeconds('')
      setMaxPerPurchase('')
      setMaxOwnedAmount('')
      setSortOrder('')
      setStartsAt('')
      setEndsAt('')
      setIsActive(true)
      setIsPublic(true)
      setMetadataText('{}')
      setCreateModalOpen(false)
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось создать товар.'))
    } finally {
      setIsCreating(false)
    }
  }

  const listCountLabel =
    items == null
      ? ''
      : productCatalogScope === 'active_only'
        ? `Найдено активных: ${displayedProducts.length}${items.length !== displayedProducts.length ? ` из ${items.length} загруженных` : ''}. Страница ${shopPage} из ${totalPages}`
        : `Найдено: ${displayedProducts.length} из ${items.length}. Страница ${shopPage} из ${totalPages}`
  const activeProductCount = items?.filter((item) => item.isActive).length ?? 0
  const hiddenProductCount = items?.filter((item) => !item.isPublic).length ?? 0
  const availableProductCount = items?.filter((item) => item.isAvailableNow).length ?? 0

  return (
    <div className="admin-shop-layout">
      <section className="card admin-card admin-shop-list">
        <div className="admin-section-head">
          <div>
            <h2 className="card-title">Магазин</h2>
            <p className="card-text">Товары витрины, цены, локали, расписание доступности и привязанные ассеты.</p>
          </div>
          <div className="admin-shop-list-toolbar">
            <button type="button" className="btn btn-sm" onClick={() => void loadProducts('')}>
              Обновить
            </button>
            <button type="button" className="btn primary btn-sm" onClick={() => setCreateModalOpen(true)}>
              Создать товар…
            </button>
          </div>
        </div>

        {items ? (
          <div className="admin-metric-strip">
            <div className="admin-metric">
              <span>Всего</span>
              <strong>{items.length}</strong>
            </div>
            <div className="admin-metric admin-metric--success">
              <span>Активны</span>
              <strong>{activeProductCount}</strong>
            </div>
            <div className="admin-metric admin-metric--warning">
              <span>Скрыты</span>
              <strong>{hiddenProductCount}</strong>
            </div>
          </div>
        ) : null}

        <div className="admin-shop-list-controls admin-filter-bar">
          <div className="admin-shop-list-controls-row">
            <label className="admin-shop-field admin-filter-label">
              <span>Поиск</span>
              <input
                className="ui-input"
                value={shopQuery}
                onChange={(event) => setShopQueryAndReset(event.target.value)}
                placeholder="Название, key или asset"
              />
            </label>
            <label className="admin-shop-field admin-filter-label">
              <span>Сортировка</span>
              <select className="ui-input" value={shopSort} onChange={(e) => setShopSort(e.target.value as ShopAdminSortKey)} aria-label="Сортировка товаров">
                <option value="catalog">Как в каталоге (sort_order)</option>
                <option value="key">Ключ товара (A–Я)</option>
                <option value="price_asc">Цена ↑</option>
                <option value="price_desc">Цена ↓</option>
                <option value="name">Название (A–Я)</option>
                <option value="updated_desc">Обновление (новые сверху)</option>
              </select>
            </label>
            <fieldset className="ui-radio-group admin-asset-flag-fieldset">
              <legend className="ui-radio-legend">Состав списка</legend>
              <label className="ui-radio">
                <input
                  type="radio"
                  name="admin-shop-catalog-scope"
                  checked={productCatalogScope === 'active_only'}
                  onChange={() => setProductCatalogScopeAndReset('active_only')}
                />
                <span className="ui-radio-mark" aria-hidden />
                <span>Только активные</span>
              </label>
              <label className="ui-radio">
                <input
                  type="radio"
                  name="admin-shop-catalog-scope"
                  checked={productCatalogScope === 'all'}
                  onChange={() => setProductCatalogScopeAndReset('all')}
                />
                <span className="ui-radio-mark" aria-hidden />
                <span>Все товары</span>
              </label>
            </fieldset>
          </div>
          {listCountLabel ? <p className="admin-inline-muted admin-zero-margin">{listCountLabel}</p> : null}
        </div>

        {listError ? <ErrorState message={listError} /> : null}
        {!items && !listError ? <LoadingState title="Загружаем товары" /> : null}
        {items && items.length === 0 ? <p className="admin-inline-muted">Товаров пока нет.</p> : null}
        {items && items.length > 0 && displayedProducts.length === 0 ? (
          <p className="admin-inline-muted">Нет активных товаров. Включите «Все товары», чтобы увидеть выключенные позиции.</p>
        ) : null}
        {displayedProducts.length > 0 ? (
          <div className="admin-list">
            {pagedProducts.map((p) => (
              <AdminLink key={p.id} href={adminShopProductPath(p.id)} className="admin-row">
                <span className="admin-row-user">
                  <span className="admin-row-user-text">
                    <strong>{p.localizedName}</strong>
                    <small>
                      {p.key} · {p.assetKey} · {p.priceRub} ₽ · {p.ownershipModel}
                    </small>
                  </span>
                </span>
                <div className="admin-row-badges">
                  <span className={`ui-badge ${p.isActive ? 'ui-badge-success' : 'ui-badge-warning'}`}>{p.isActive ? 'Активен' : 'Выкл'}</span>
                  <span className={`ui-badge ${p.isPublic ? 'ui-badge-neutral' : 'ui-badge-secondary'}`}>{p.isPublic ? 'Публичный' : 'Скрыт'}</span>
                </div>
              </AdminLink>
            ))}
          </div>
        ) : null}
        {displayedProducts.length > 0 ? (
          <AdminPagination page={shopPage} totalPages={totalPages} onPageChange={setShopPage} label="Нумерация страниц товаров" />
        ) : null}
      </section>

      {createModalOpen ? (
        <AppPortal>
          <div
            className="ui-modal-backdrop"
            role="presentation"
            onClick={() => {
              if (!isCreating) setCreateModalOpen(false)
            }}
          >
            <div className="ui-modal admin-create-modal" role="dialog" aria-modal="true" aria-labelledby="admin-create-shop-title" onClick={(e) => e.stopPropagation()}>
              <div className="ui-modal-header">
                <h2 id="admin-create-shop-title" className="ui-modal-title">Новый товар</h2>
                <button type="button" className="ui-modal-close" aria-label="Закрыть" disabled={isCreating} onClick={() => setCreateModalOpen(false)}>
                  ×
                </button>
              </div>
              <div className="ui-modal-body">
                <p className="card-text admin-inline-muted admin-modal-note">
                  Выберите ассет, и форма покажет только поля, которые принимает его модель владения.
                </p>
                <div className="admin-shop-form-grid">
                  <label className="admin-shop-field">
                    <span>Ключ товара (key)</span>
                    <input className="ui-input" value={productKey} onChange={(e) => setProductKey(e.target.value)} placeholder="vip_month" />
                  </label>
                  <label className="admin-shop-field admin-shop-field--asset-select">
                    <span>Ассет (ещё без товара в магазине)</span>
                    {assetsError ? <span className="admin-inline-muted">{assetsError}</span> : null}
                    {!assetsError && catalogAssets === null ? (
                      <span className="admin-inline-muted">Загружаем каталог ассетов…</span>
                    ) : null}
                    {!assetsError && catalogAssets ? (
                      <select className="ui-input" value={assetKey} onChange={(e) => setAssetKey(e.target.value)} aria-label="Ключ ассета">
                        <option value="">— выберите ассет —</option>
                        {pickableAssets.map((a) => (
                          <option key={a.id} value={a.key}>
                            {a.displayName} ({a.key}) · {a.ownershipModel}
                          </option>
                        ))}
                      </select>
                    ) : null}
                    {!assetsError && catalogAssets && pickableAssets.length === 0 ? (
                      <span className="admin-inline-muted">Нет свободных невалютных ассетов: все уже привязаны к товарам или отсутствуют в каталоге.</span>
                    ) : null}
                  </label>
                  <label className="admin-shop-field">
                    <span>Цена, ₽ (price_rub)</span>
                    <input className="ui-input" type="number" min={0} step={1} value={priceRub} onChange={(e) => setPriceRub(e.target.value)} placeholder="499" />
                  </label>
                  <label className="admin-shop-field">
                    <span>sort_order</span>
                    <input className="ui-input" type="number" value={sortOrder} onChange={(e) => setSortOrder(e.target.value)} placeholder="опционально" />
                  </label>
                  {selectedAsset?.ownershipModel === 'stackable' ? (
                    <>
                      <label className="admin-shop-field">
                        <span>stackable_amount</span>
                        <input className="ui-input" type="number" min={1} value={stackableAmount} onChange={(e) => setStackableAmount(e.target.value)} placeholder="например 10" />
                        <small>Обязательное поле: сколько единиц stackable-ассета начисляется за одну покупку.</small>
                      </label>
                      <label className="admin-shop-field">
                        <span>max_per_purchase</span>
                        <input className="ui-input" type="number" min={1} value={maxPerPurchase} onChange={(e) => setMaxPerPurchase(e.target.value)} placeholder="опционально" />
                        <small>Только для stackable: максимум единиц товара за одну покупку.</small>
                      </label>
                      <label className="admin-shop-field">
                        <span>max_owned_amount</span>
                        <input className="ui-input" type="number" min={1} value={maxOwnedAmount} onChange={(e) => setMaxOwnedAmount(e.target.value)} placeholder="опционально" />
                        <small>Только для stackable: максимум единиц ассета у игрока после покупки.</small>
                      </label>
                    </>
                  ) : null}
                  {selectedAsset?.ownershipModel === 'expirable' ? (
                    <label className="admin-shop-field">
                      <span>durationSeconds</span>
                      <input className="ui-input" type="number" min={1} value={durationSeconds} onChange={(e) => setDurationSeconds(e.target.value)} placeholder="например 2592000" />
                      <small>Обязательное поле: срок действия expirable-ассета в секундах.</small>
                    </label>
                  ) : null}
                  {selectedAsset?.ownershipModel === 'entitlement' ? (
                    <p className="admin-field-hint admin-shop-field--full">
                      Для entitlement-товара дополнительных ownership-полей нет: сервер принимает только цену, локали, флаги и расписание.
                    </p>
                  ) : null}
                  <label className="admin-shop-field">
                    <span>starts_at (ISO 8601)</span>
                    <input className="ui-input" value={startsAt} onChange={(e) => setStartsAt(e.target.value)} placeholder="2026-01-01T00:00:00Z" />
                  </label>
                  <label className="admin-shop-field">
                    <span>ends_at (ISO 8601)</span>
                    <input className="ui-input" value={endsAt} onChange={(e) => setEndsAt(e.target.value)} />
                  </label>
                </div>

                <div className="admin-asset-flags admin-shop-form-flags admin-asset-flags--radios">
                  <fieldset className="ui-radio-group admin-asset-flag-fieldset">
                    <legend className="ui-radio-legend">Статус</legend>
                    <label className="ui-radio">
                      <input type="radio" name="admin-create-shop-active" checked={isActive} onChange={() => setIsActive(true)} />
                      <span className="ui-radio-mark" aria-hidden />
                      <span>Активен</span>
                    </label>
                    <label className="ui-radio">
                      <input type="radio" name="admin-create-shop-active" checked={!isActive} onChange={() => setIsActive(false)} />
                      <span className="ui-radio-mark" aria-hidden />
                      <span>Неактивен</span>
                    </label>
                  </fieldset>
                  <fieldset className="ui-radio-group admin-asset-flag-fieldset">
                    <legend className="ui-radio-legend">Видимость</legend>
                    <label className="ui-radio">
                      <input type="radio" name="admin-create-shop-public" checked={isPublic} onChange={() => setIsPublic(true)} />
                      <span className="ui-radio-mark" aria-hidden />
                      <span>Публичный</span>
                    </label>
                    <label className="ui-radio">
                      <input type="radio" name="admin-create-shop-public" checked={!isPublic} onChange={() => setIsPublic(false)} />
                      <span className="ui-radio-mark" aria-hidden />
                      <span>Скрытый</span>
                    </label>
                  </fieldset>
                </div>

                <div className="admin-shop-locales">
                  <div className="admin-shop-locales-head">
                    <strong>Локали</strong>
                    <button type="button" className="btn btn-sm" onClick={addLocaleRow}>
                      Добавить локаль
                    </button>
                  </div>
                  {localeRows.map((row, index) => (
                    <div key={index} className="admin-shop-locale-row">
                      <input className="ui-input" value={row.locale} onChange={(e) => updateLocaleRow(index, { locale: e.target.value })} placeholder="ru-RU" aria-label="Код локали" />
                      <input className="ui-input" value={row.name} onChange={(e) => updateLocaleRow(index, { name: e.target.value })} placeholder="Название" />
                      <textarea className="ui-input" value={row.description} onChange={(e) => updateLocaleRow(index, { description: e.target.value })} placeholder="Описание" />
                      <button type="button" className="btn btn-sm danger" disabled={localeRows.length <= 1} onClick={() => removeLocaleRow(index)}>
                        Удалить локаль
                      </button>
                    </div>
                  ))}
                </div>

                <label className="admin-shop-field admin-shop-field--full">
                  <span>metadata (JSON)</span>
                  <textarea className="ui-input admin-shop-textarea" rows={4} value={metadataText} onChange={(e) => setMetadataText(e.target.value)} spellCheck={false} />
                </label>
              </div>
              <div className="ui-modal-footer">
                <button type="button" className="btn btn-sm" disabled={isCreating} onClick={() => setCreateModalOpen(false)}>
                  Отмена
                </button>
                <button type="button" className="btn primary btn-sm" disabled={isCreating} onClick={() => void createProduct()}>
                  {isCreating ? 'Создаём…' : 'Создать товар'}
                </button>
              </div>
            </div>
          </div>
        </AppPortal>
      ) : null}
    </div>
  )
}
