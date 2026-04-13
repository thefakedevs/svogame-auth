import { useCallback, useEffect, useMemo, useState } from 'react'
import toast from 'react-hot-toast'
import { createAdminShopProduct, listAdminShopProducts, type CreateShopProductInput } from '../../api/admin'
import { toDisplayError } from '../../api/http'
import { listAllAdminAssets, type AssetResponse } from '../../api/inventory'
import type { ShopProductResponse } from '../../api/shop'
import { adminShopProductPath } from '../../routes/paths'
import { pushUrl } from '../../shared/navigation/history'
import ErrorState from '../ErrorState'
import LoadingState from '../LoadingState'

type LocaleRow = { locale: string; name: string; description: string }

const defaultLocaleRows: LocaleRow[] = [{ locale: 'ru-RU', name: '', description: '' }]

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
    isActive: boolean | null
    isPublic: boolean | null
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

  if (extras.isActive !== null) body.is_active = extras.isActive
  if (extras.isPublic !== null) body.is_public = extras.isPublic

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
  const [isActive, setIsActive] = useState<boolean | null>(true)
  const [isPublic, setIsPublic] = useState<boolean | null>(true)
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
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось создать товар.'))
    } finally {
      setIsCreating(false)
    }
  }

  return (
    <div className="admin-shop-layout">
      <section className="card admin-card admin-shop-create">
        <h2 className="card-title">Новый товар</h2>
        <p className="card-text admin-inline-muted">
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
          <label className="admin-shop-field admin-shop-field--checkbox">
            <span>is_active</span>
            <select
              className="ui-input"
              value={isActive === null ? '' : isActive ? 'true' : 'false'}
              onChange={(e) => {
                const v = e.target.value
                setIsActive(v === '' ? null : v === 'true')
              }}
            >
              <option value="true">true</option>
              <option value="false">false</option>
              <option value="">не передавать</option>
            </select>
          </label>
          <label className="admin-shop-field admin-shop-field--checkbox">
            <span>is_public</span>
            <select
              className="ui-input"
              value={isPublic === null ? '' : isPublic ? 'true' : 'false'}
              onChange={(e) => {
                const v = e.target.value
                setIsPublic(v === '' ? null : v === 'true')
              }}
            >
              <option value="true">true</option>
              <option value="false">false</option>
              <option value="">не передавать</option>
            </select>
          </label>
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
              <input className="ui-input" value={row.description} onChange={(e) => updateLocaleRow(index, { description: e.target.value })} placeholder="Описание" />
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

        <div className="admin-shop-actions">
          <button type="button" className="btn primary" disabled={isCreating} onClick={() => void createProduct()}>
            {isCreating ? 'Создаём…' : 'Создать товар'}
          </button>
        </div>
      </section>

      <section className="card admin-card admin-shop-list">
        <div className="admin-shop-list-head">
          <h2 className="card-title">Каталог</h2>
          <div className="admin-shop-list-toolbar">
            <input
              className="ui-input"
              value={listLocaleInput}
              onChange={(e) => setListLocaleInput(e.target.value)}
              placeholder="locale для списка (опционально)"
            />
            <button type="button" className="btn btn-sm" onClick={() => void loadProducts(listLocaleInput)}>
              Обновить
            </button>
          </div>
        </div>
        {listError ? <ErrorState message={listError} /> : null}
        {!items && !listError ? <LoadingState title="Загружаем товары" /> : null}
        {items && items.length === 0 ? <p className="admin-inline-muted">Товаров пока нет.</p> : null}
        {items && items.length > 0 ? (
          <div className="admin-list">
            {items.map((p) => (
              <button key={p.id} type="button" className="admin-row" onClick={() => pushUrl(adminShopProductPath(p.id))}>
                <span className="admin-row-user">
                  <span className="admin-row-user-text">
                    <strong>{p.localizedName}</strong>
                    <small>
                      {p.key} · {p.assetKey} · {p.priceRub} ₽ · {p.ownershipModel}
                    </small>
                  </span>
                </span>
                <span className={`ui-badge ${p.isActive ? 'ui-badge-success' : 'ui-badge-warning'}`}>{p.isActive ? 'Активен' : 'Выкл'}</span>
                <span className={`ui-badge ${p.isPublic ? 'ui-badge-neutral' : 'ui-badge-secondary'}`}>{p.isPublic ? 'Публичный' : 'Скрыт'}</span>
              </button>
            ))}
          </div>
        ) : null}
      </section>
    </div>
  )
}
