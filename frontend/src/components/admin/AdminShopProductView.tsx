import { useEffect, useState } from 'react'
import toast from 'react-hot-toast'
import { getAdminShopProduct, patchAdminShopProduct, type UpdateShopProductInput } from '../../api/admin'
import { toDisplayError } from '../../api/http'
import type { ShopProductResponse } from '../../api/shop'
import { paths } from '../../routes/paths'
import { pushUrl } from '../../shared/navigation/history'
import ErrorState from '../ErrorState'
import LoadingState from '../LoadingState'

type LocaleRow = { locale: string; name: string; description: string }

function rowsFromProduct(product: ShopProductResponse): LocaleRow[] {
  return product.locales.map((l) => ({
    locale: l.locale,
    name: l.name,
    description: l.description ?? '',
  }))
}

function metadataToText(value: unknown) {
  if (value == null) return '{}'
  try {
    return JSON.stringify(value, null, 2)
  } catch {
    return String(value)
  }
}

export default function AdminShopProductView({ token, productId }: { token: string; productId: string }) {
  const [product, setProduct] = useState<ShopProductResponse | null>(null)
  const [loadError, setLoadError] = useState('')
  const [localeForFetch, setLocaleForFetch] = useState('')
  const [localeInputDraft, setLocaleInputDraft] = useState('')

  const [priceRub, setPriceRub] = useState('')
  const [localeRows, setLocaleRows] = useState<LocaleRow[]>([])
  const [stackableAmount, setStackableAmount] = useState('')
  const [durationSeconds, setDurationSeconds] = useState('')
  const [maxPerPurchase, setMaxPerPurchase] = useState('')
  const [maxOwnedAmount, setMaxOwnedAmount] = useState('')
  const [sortOrder, setSortOrder] = useState('')
  const [startsAt, setStartsAt] = useState('')
  const [endsAt, setEndsAt] = useState('')
  const [isActive, setIsActive] = useState(false)
  const [isPublic, setIsPublic] = useState(false)
  const [metadataText, setMetadataText] = useState('{}')
  const [isSaving, setIsSaving] = useState(false)

  useEffect(() => {
    let cancelled = false
    const run = async () => {
      setLoadError('')
      setProduct(null)
      try {
        const p = await getAdminShopProduct(token, productId, localeForFetch.trim() || undefined)
        if (cancelled) return
        setProduct(p)
        setPriceRub(String(p.priceRub))
        setLocaleRows(rowsFromProduct(p))
        setStackableAmount(p.stackableAmount != null ? String(p.stackableAmount) : '')
        setDurationSeconds(p.durationSeconds != null ? String(p.durationSeconds) : '')
        setMaxPerPurchase(p.maxPerPurchase != null ? String(p.maxPerPurchase) : '')
        setMaxOwnedAmount(p.maxOwnedAmount != null ? String(p.maxOwnedAmount) : '')
        setSortOrder(String(p.sortOrder))
        setStartsAt(p.startsAt ?? '')
        setEndsAt(p.endsAt ?? '')
        setIsActive(p.isActive)
        setIsPublic(p.isPublic)
        setMetadataText(metadataToText(p.metadata))
      } catch (cause) {
        if (!cancelled) setLoadError(toDisplayError(cause, 'Не удалось загрузить товар.'))
      }
    }
    void run()
    return () => {
      cancelled = true
    }
  }, [token, productId, localeForFetch])

  const updateLocaleRow = (index: number, patch: Partial<LocaleRow>) => {
    setLocaleRows((prev) => prev.map((row, i) => (i === index ? { ...row, ...patch } : row)))
  }

  const addLocaleRow = () => {
    setLocaleRows((prev) => [...prev, { locale: '', name: '', description: '' }])
  }

  const removeLocaleRow = (index: number) => {
    setLocaleRows((prev) => (prev.length <= 1 ? prev : prev.filter((_, i) => i !== index)))
  }

  const save = async () => {
    if (!product) return
    const price = Number(priceRub)
    if (!Number.isFinite(price) || price < 0) {
      toast.error('Укажите корректную цену в рублях.')
      return
    }
    let metadata: unknown
    try {
      const trimmed = metadataText.trim()
      metadata = trimmed ? JSON.parse(trimmed) : null
    } catch {
      toast.error('Некорректный JSON в metadata.')
      return
    }

    const locales = localeRows
      .map((row) => ({
        locale: row.locale.trim(),
        name: row.name.trim(),
        description: row.description.trim() || null,
      }))
      .filter((row) => row.locale && row.name)

    if (!locales.length) {
      toast.error('Нужна хотя бы одна локаль с кодом и названием.')
      return
    }

    const sortParsed = Number.parseInt(sortOrder, 10)
    const sortOrderValue = Number.isFinite(sortParsed) ? sortParsed : product.sortOrder

    const patch: UpdateShopProductInput = {
      price_rub: price,
      locales,
      is_active: isActive,
      is_public: isPublic,
      sort_order: sortOrderValue,
      metadata,
    }

    const stack = stackableAmount.trim()
    patch.stackable_amount = stack ? Number(stack) : null

    const dur = durationSeconds.trim()
    patch.durationSeconds = dur ? Number(dur) : null

    const mpp = maxPerPurchase.trim()
    patch.max_per_purchase = mpp ? Number(mpp) : null

    const moa = maxOwnedAmount.trim()
    patch.max_owned_amount = moa ? Number(moa) : null

    patch.starts_at = startsAt.trim() || null
    patch.ends_at = endsAt.trim() || null

    setIsSaving(true)
    try {
      const updated = await patchAdminShopProduct(token, productId, patch)
      setProduct(updated)
      setLocaleRows(rowsFromProduct(updated))
      setMetadataText(metadataToText(updated.metadata))
      toast.success('Товар обновлён.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось сохранить изменения.'))
    } finally {
      setIsSaving(false)
    }
  }

  return (
    <div className="admin-page">
      <section className="admin-top-actions">
        <button type="button" className="btn btn-sm" onClick={() => pushUrl(`${paths.admin}?tab=shop`)}>
          ← К магазину
        </button>
      </section>

      <section className="card admin-card">
        {!product && !loadError ? <LoadingState title="Загружаем товар" /> : null}
        {loadError ? <ErrorState message={loadError} /> : null}
        {product ? (
          <>
            <h2 className="card-title">{product.localizedName}</h2>
            <p className="card-text">
              <code>{product.key}</code> · ID: <code>{product.id}</code>
            </p>
            <dl className="admin-kv admin-kv--compact">
              <div>
                <dt>Ассет</dt>
                <dd>
                  {product.assetDisplayName} (<code>{product.assetKey}</code>)
                </dd>
              </div>
              <div>
                <dt>Модель владения</dt>
                <dd>{product.ownershipModel}</dd>
              </div>
              <div>
                <dt>Витрина</dt>
                <dd>{product.isAvailableNow ? 'Доступен сейчас' : 'Недоступен по расписанию/флагам'}</dd>
              </div>
            </dl>

            <div className="admin-shop-list-toolbar" style={{ marginTop: '1rem' }}>
              <label className="admin-shop-field" style={{ flex: 1, marginBottom: 0 }}>
                <span className="admin-inline-muted">Параметр locale при загрузке (опционально)</span>
                <input className="ui-input" value={localeInputDraft} onChange={(e) => setLocaleInputDraft(e.target.value)} placeholder="ru-RU" />
              </label>
              <button type="button" className="btn btn-sm" onClick={() => setLocaleForFetch(localeInputDraft)}>
                Загрузить с локалью
              </button>
            </div>

            <div className="admin-shop-form-grid" style={{ marginTop: '1.25rem' }}>
              <label className="admin-shop-field">
                <span>Цена, ₽</span>
                <input className="ui-input" type="number" min={0} step={1} value={priceRub} onChange={(e) => setPriceRub(e.target.value)} />
              </label>
              <label className="admin-shop-field">
                <span>sort_order</span>
                <input className="ui-input" type="number" value={sortOrder} onChange={(e) => setSortOrder(e.target.value)} />
              </label>
              <label className="admin-shop-field">
                <span>stackable_amount</span>
                <input className="ui-input" type="number" min={1} value={stackableAmount} onChange={(e) => setStackableAmount(e.target.value)} />
              </label>
              <label className="admin-shop-field">
                <span>durationSeconds</span>
                <input className="ui-input" type="number" min={1} value={durationSeconds} onChange={(e) => setDurationSeconds(e.target.value)} />
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
                <span>starts_at</span>
                <input className="ui-input" value={startsAt} onChange={(e) => setStartsAt(e.target.value)} />
              </label>
              <label className="admin-shop-field">
                <span>ends_at</span>
                <input className="ui-input" value={endsAt} onChange={(e) => setEndsAt(e.target.value)} />
              </label>
              <label className="admin-shop-field admin-shop-field--checkbox">
                <span>is_active</span>
                <input type="checkbox" checked={isActive} onChange={(e) => setIsActive(e.target.checked)} />
              </label>
              <label className="admin-shop-field admin-shop-field--checkbox">
                <span>is_public</span>
                <input type="checkbox" checked={isPublic} onChange={(e) => setIsPublic(e.target.checked)} />
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
                  <input className="ui-input" value={row.locale} onChange={(e) => updateLocaleRow(index, { locale: e.target.value })} placeholder="ru-RU" />
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
              <textarea className="ui-input admin-shop-textarea" rows={5} value={metadataText} onChange={(e) => setMetadataText(e.target.value)} spellCheck={false} />
            </label>

            <div className="admin-shop-actions">
              <button type="button" className="btn primary" disabled={isSaving} onClick={() => void save()}>
                {isSaving ? 'Сохраняем…' : 'Сохранить'}
              </button>
            </div>
          </>
        ) : null}
      </section>
    </div>
  )
}
