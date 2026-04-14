import { useCallback, useEffect, useMemo, useState } from 'react'
import toast from 'react-hot-toast'
import { createAdminShopProduct, listAdminShopProducts, type CreateShopProductInput } from '../../api/admin'
import { toDisplayError } from '../../api/http'
import { listAllAdminAssets, type AssetResponse } from '../../api/inventory'
import type { ShopProductResponse } from '../../api/shop'
import { adminShopProductPath } from '../../routes/paths'
import AppPortal from '../../shared/ui/portal/AppPortal'
import { pushUrl } from '../../shared/navigation/history'
import ErrorState from '../ErrorState'
import LoadingState from '../LoadingState'
import './AdminShopPanel.css'

type LocaleRow = { locale: string; name: string; description: string }

const defaultLocaleRows: LocaleRow[] = [{ locale: 'ru-RU', name: '', description: '' }]

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

function buildCreatePayload(
  key: string,
  assetKey: string,
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
  if (stack) body.stackable_amount = Number(stack)

  const dur = extras.durationSeconds.trim()
  if (dur) body.durationSeconds = Number(dur)

  const mpp = extras.maxPerPurchase.trim()
  if (mpp) body.max_per_purchase = Number(mpp)

  const moa = extras.maxOwnedAmount.trim()
  if (moa) body.max_owned_amount = Number(moa)

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
  const [listLocaleInput, setListLocaleInput] = useState('')

  const [catalogAssets, setCatalogAssets] = useState<AssetResponse[] | null>(null)
  const [assetsError, setAssetsError] = useState('')

  const [createModalOpen, setCreateModalOpen] = useState(false)
  const [productCatalogScope, setProductCatalogScope] = useState<'active_only' | 'all'>('active_only')
  const [shopSort, setShopSort] = useState<ShopAdminSortKey>('catalog')

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

  useEffect(() => {
    if (!assetKey || !catalogAssets) return
    const allowed = new Set(pickableAssets.map((a) => a.key))
    if (!allowed.has(assetKey)) setAssetKey('')
  }, [assetKey, catalogAssets, pickableAssets])

  const displayedProducts = useMemo(() => {
    if (!items) return []
    const filtered = productCatalogScope === 'active_only' ? items.filter((p) => p.isActive) : items
    return sortAdminShopProducts(filtered, shopSort)
  }, [items, productCatalogScope, shopSort])

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
    let payload: CreateShopProductInput
    try {
      payload = buildCreatePayload(productKey, assetKey, price, localeRows, {
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
      await loadProducts(listLocaleInput)
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
        ? `Показано активных: ${displayedProducts.length}${items.length !== displayedProducts.length ? ` из ${items.length} загруженных` : ''}`
        : `Всего: ${items.length}`

  return (
    <div className="admin-shop-layout">
      <section className="card admin-card admin-shop-list">
        <div className="admin-shop-list-head">
          <h2 className="card-title">Каталог</h2>
          <div className="admin-shop-list-toolbar">
            <button type="button" className="btn btn-sm" onClick={() => void loadProducts(listLocaleInput)}>
              Обновить
            </button>
            <button type="button" className="btn primary btn-sm" onClick={() => setCreateModalOpen(true)}>
              Создать товар…
            </button>
          </div>
        </div>

        <div className="admin-shop-list-controls">
          <div className="admin-shop-list-controls-row">
            <label className="admin-shop-field" style={{ marginBottom: 0 }}>
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
                  onChange={() => setProductCatalogScope('active_only')}
                />
                <span className="ui-radio-mark" aria-hidden />
                <span>Только активные</span>
              </label>
              <label className="ui-radio">
                <input
                  type="radio"
                  name="admin-shop-catalog-scope"
                  checked={productCatalogScope === 'all'}
                  onChange={() => setProductCatalogScope('all')}
                />
                <span className="ui-radio-mark" aria-hidden />
                <span>Все товары</span>
              </label>
            </fieldset>
          </div>
          {listCountLabel ? <p className="admin-inline-muted" style={{ margin: 0 }}>{listCountLabel}</p> : null}
        </div>

        {listError ? <ErrorState message={listError} /> : null}
        {!items && !listError ? <LoadingState title="Загружаем товары" /> : null}
        {items && items.length === 0 ? <p className="admin-inline-muted">Товаров пока нет.</p> : null}
        {items && items.length > 0 && displayedProducts.length === 0 ? (
          <p className="admin-inline-muted">Нет активных товаров. Включите «Все товары», чтобы увидеть выключенные позиции.</p>
        ) : null}
        {displayedProducts.length > 0 ? (
          <div className="admin-list">
            {displayedProducts.map((p) => (
              <button key={p.id} type="button" className="admin-row" onClick={() => pushUrl(adminShopProductPath(p.id))}>
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
              </button>
            ))}
          </div>
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
                <p className="card-text admin-inline-muted" style={{ marginTop: 0 }}>
                  Привязка к существующему невалютному ассету. Для stackable укажите количество, для подписки — длительность в секундах; для entitlement не заполняйте оба поля.
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
                  <label className="admin-shop-field">
                    <span>stackable_amount</span>
                    <input className="ui-input" type="number" min={1} value={stackableAmount} onChange={(e) => setStackableAmount(e.target.value)} placeholder="для stackable" />
                  </label>
                  <label className="admin-shop-field">
                    <span>durationSeconds</span>
                    <input className="ui-input" type="number" min={1} value={durationSeconds} onChange={(e) => setDurationSeconds(e.target.value)} placeholder="для expirable" />
                  </label>
                  <label className="admin-shop-field">
                    <span>max_per_purchase</span>
                    <input className="ui-input" type="number" min={1} value={maxPerPurchase} onChange={(e) => setMaxPerPurchase(e.target.value)} />
                  </label>
                  <label className="admin-shop-field">
                    <span>max_owned_amount</span>
                    <input className="ui-input" type="number" min={1} value={maxOwnedAmount} onChange={(e) => setMaxOwnedAmount(e.target.value)} />
                  </label>
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
                        Удалить
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
