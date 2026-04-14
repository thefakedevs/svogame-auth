import { useEffect, useMemo, useState, type CSSProperties, type ReactNode } from 'react'
import toast from 'react-hot-toast'
import { toDisplayError } from '../api/http'
import {
  buildPublicAssetImageUrl,
  listMyEntitlements,
  listPublicAssets,
  type AssetResponse,
  type EntitlementResponse,
  type SkinRarity,
} from '../api/inventory'
import { createMyShopOrder, listPublicShopProducts, type ShopProductResponse } from '../api/shop'
import { getCurrentUser } from '../api/users'
import ErrorState from '../components/ErrorState'
import LoadingState from '../components/LoadingState'
import SkinDetailsModal from '../components/SkinDetailsModal'
import { currentAppPath, redirectToAuth } from '../routes/auth'
import { getAuthToken } from '../shared/session/auth-session'
import AppPortal from '../shared/ui/portal/AppPortal'
import './OwnershipPage.css'
import './ShopPage.css'
import { skinRarityRank, uniqueSortedWeaponKeys } from './skinInventoryControls'

const SHOP_LOCALE = 'ru-RU'

type ShopSortKey = 'default' | 'price_asc' | 'price_desc' | 'rarity' | 'weapon'
type ShopRarityFilter = 'all' | 'none' | SkinRarity
type ShopWeaponFilter = 'all' | string

type ShopState =
  | { status: 'loading' }
  | { status: 'error'; error: string }
  | { status: 'ready'; products: ShopProductResponse[]; assets: AssetResponse[]; entitlements: EntitlementResponse[] }

const skinRarityConfig: Record<SkinRarity, { label: string; color: string }> = {
  common: { label: 'Обычный', color: '#9aa8b5' },
  rare: { label: 'Редкий', color: '#2f8cff' },
  legendary: { label: 'Легендарный', color: '#ffb02e' },
}

const priceFormatter = new Intl.NumberFormat('ru-RU', {
  style: 'currency',
  currency: 'RUB',
  maximumFractionDigits: 0,
})

function formatPrice(value: number) {
  return priceFormatter.format(value)
}

function cardStyle(accent: string): CSSProperties {
  return { '--inventory-accent': accent } as CSSProperties
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

function assetImageUrl(asset: AssetResponse | null) {
  if (!asset) return null
  return asset.imageUrl
    ?? metadataString(asset, ['imageUrl', 'image', 'iconUrl', 'icon', 'thumbnailUrl', 'thumbnail', 'previewUrl', 'skinUrl'])
    ?? buildPublicAssetImageUrl(asset.id, asset.updatedAt)
}

function fallbackAccent(key: string) {
  const palette = ['#2db7a3', '#f7a41d', '#67d391', '#ff8b8b', '#6fb6ff', '#d6a4ff']
  let hash = 0
  for (const char of key) hash = (hash + char.charCodeAt(0)) % palette.length
  return palette[hash]
}

function productAccent(product: ShopProductResponse, asset: AssetResponse | null) {
  return asset?.rarity
    ? skinRarityConfig[asset.rarity].color
    : metadataString(asset, ['accentColor', 'color', 'rarityColor'])
      ?? metadataString(product, ['accentColor', 'color', 'rarityColor'])
      ?? fallbackAccent(product.assetKey)
}

function rarityLabel(rarity: SkinRarity | null | undefined) {
  return rarity ? skinRarityConfig[rarity].label : 'Скин'
}

function buildAssetMap(assets: AssetResponse[]) {
  const map = new Map<string, AssetResponse>()
  for (const asset of assets) {
    map.set(asset.key, asset)
    map.set(asset.id, asset)
  }
  return map
}

function buildOwnedSet(entitlements: EntitlementResponse[]) {
  const set = new Set<string>()
  for (const item of entitlements) {
    set.add(item.assetKey)
    set.add(item.assetDefinitionId)
  }
  return set
}

function isOwned(product: ShopProductResponse, ownedSet: Set<string>) {
  return ownedSet.has(product.assetKey) || ownedSet.has(product.assetDefinitionId)
}

function productDescription(product: ShopProductResponse, asset: AssetResponse | null) {
  return product.localizedDescription
    ?? asset?.description
    ?? metadataString(product, ['description', 'details'])
    ?? metadataString(asset, ['description', 'details'])
    ?? 'Скин для твоего инвентаря.'
}

function isSkinProduct(product: ShopProductResponse, asset: AssetResponse | null) {
  if (product.ownershipModel !== 'entitlement') return false
  if (!asset) return true
  if (asset.assetKind === 'skin' || asset.weaponKey) return true

  const haystack = [
    product.key,
    product.assetKey,
    product.localizedName,
    product.assetDisplayName,
    asset.key,
    asset.displayName,
    asset.assetKind,
  ].join(' ').toLowerCase()

  return haystack.includes('skin') || haystack.includes('скин')
}

async function listAllEntitlementAssets(perPage = 100) {
  const items: AssetResponse[] = []
  let page = 1

  while (true) {
    const response = await listPublicAssets({ ownershipModel: 'entitlement', page, perPage })
    items.push(...response.items)

    if (items.length >= response.total || response.items.length === 0) {
      return items
    }

    page += 1
  }
}

type ShopProductView = {
  product: ShopProductResponse
  asset: AssetResponse | null
  title: string
  description: string
  imageUrl: string | null
  accent: string
  owned: boolean
}

function InventoryVisual({ item }: { item: { title: string; imageUrl: string | null; accent: string } }) {
  const [failedImageUrl, setFailedImageUrl] = useState<string | null>(null)
  const fallback = item.title.trim().slice(0, 1).toUpperCase() || 'S'
  const showImage = Boolean(item.imageUrl) && failedImageUrl !== item.imageUrl

  return (
    <div className="inventory-visual" style={cardStyle(item.accent)}>
      {showImage ? <img src={item.imageUrl ?? ''} alt="" loading="lazy" onError={() => setFailedImageUrl(item.imageUrl)} /> : <span>{fallback}</span>}
      <div className="inventory-visual-splash" />
    </div>
  )
}

function ProductCard({
  item,
  isBuying,
  onOpenDetails,
  onBuy,
}: {
  item: ShopProductView
  isBuying: boolean
  onOpenDetails: (item: ShopProductView) => void
  onBuy: (item: ShopProductView) => void
}) {
  return (
    <article
      className={`inventory-skin-card shop-product-card ${item.owned ? 'is-owned' : ''}`}
      data-rarity={item.asset?.rarity ?? 'none'}
      style={cardStyle(item.accent)}
    >
      <button className="shop-product-card__preview" type="button" onClick={() => onOpenDetails(item)}>
        <InventoryVisual item={item} />
      </button>
      <div className="inventory-skin-card__body">
        <div className="inventory-card-title">
          <strong>{item.title}</strong>
        </div>
        <div className="inventory-skin-card__rarity">
          <span>{rarityLabel(item.asset?.rarity)}</span>
        </div>
        <div className="shop-product-card__footer">
          <strong>{formatPrice(item.product.priceRub)}</strong>
          {item.owned ? (
            <span className="ui-badge ui-badge-success">Уже есть</span>
          ) : (
            <button
              className="inventory-skin-card__action shop-product-card__buy"
              type="button"
              disabled={isBuying}
              onClick={() => onBuy(item)}
            >
              {isBuying ? 'Готовим...' : 'Купить'}
            </button>
          )}
        </div>
      </div>
    </article>
  )
}

function ShopModal({
  title,
  titleId,
  isBusy,
  children,
  footer,
  onClose,
}: {
  title: string
  titleId: string
  isBusy?: boolean
  children: ReactNode
  footer: ReactNode
  onClose: () => void
}) {
  return (
    <div className="ui-modal-backdrop" role="presentation" onClick={() => !isBusy && onClose()}>
      <div className="ui-modal shop-modal" role="dialog" aria-modal="true" aria-labelledby={titleId} onClick={(event) => event.stopPropagation()}>
        <div className="ui-modal-header">
          <h2 id={titleId} className="ui-modal-title">{title}</h2>
          <button className="ui-modal-close" type="button" aria-label="Закрыть" onClick={onClose} disabled={isBusy}>
            ×
          </button>
        </div>
        <div className="ui-modal-body">{children}</div>
        <div className="ui-modal-footer">{footer}</div>
      </div>
    </div>
  )
}

export default function ShopPage() {
  const [state, setState] = useState<ShopState>({ status: 'loading' })
  const [detailsProduct, setDetailsProduct] = useState<ShopProductView | null>(null)
  const [confirmProduct, setConfirmProduct] = useState<ShopProductView | null>(null)
  const [buyingProductKey, setBuyingProductKey] = useState<string | null>(null)
  const [isCreatingOrder, setIsCreatingOrder] = useState(false)
  const [shopSort, setShopSort] = useState<ShopSortKey>('default')
  const [shopRarityFilter, setShopRarityFilter] = useState<ShopRarityFilter>('all')
  const [shopWeaponFilter, setShopWeaponFilter] = useState<ShopWeaponFilter>('all')
  const [isShopFiltersModalOpen, setIsShopFiltersModalOpen] = useState(false)

  const load = async () => {
    setState({ status: 'loading' })
    try {
      const token = getAuthToken()
      const [products, assets, entitlements] = await Promise.all([
        listPublicShopProducts(SHOP_LOCALE),
        listAllEntitlementAssets(),
        token ? listMyEntitlements(token).catch(() => [] as EntitlementResponse[]) : Promise.resolve([] as EntitlementResponse[]),
      ])

      setState({ status: 'ready', products, assets, entitlements })
    } catch (cause) {
      setState({ status: 'error', error: toDisplayError(cause, 'Не удалось загрузить магазин.') })
    }
  }

  useEffect(() => {
    queueMicrotask(() => void load())
  }, [])

  const baseProductViews = useMemo(() => {
    if (state.status !== 'ready') return []

    const assetMap = buildAssetMap(state.assets)
    const ownedSet = buildOwnedSet(state.entitlements)

    return state.products
      .map((product) => {
        const asset = assetMap.get(product.assetKey) ?? assetMap.get(product.assetDefinitionId) ?? null
        const title = product.localizedName || asset?.displayName || product.assetDisplayName || product.key
        const description = productDescription(product, asset)

        return {
          product,
          asset,
          title,
          description,
          imageUrl: assetImageUrl(asset),
          accent: productAccent(product, asset),
          owned: isOwned(product, ownedSet),
        }
      })
      .filter((item) => isSkinProduct(item.product, item.asset))
  }, [state])

  const shopWeaponOptions = useMemo(
    () => uniqueSortedWeaponKeys(baseProductViews.map((item) => item.asset?.weaponKey)),
    [baseProductViews],
  )

  const productViews = useMemo(() => {
    let rows = baseProductViews

    if (shopRarityFilter === 'none') {
      rows = rows.filter((item) => !item.asset?.rarity)
    } else if (shopRarityFilter !== 'all') {
      rows = rows.filter((item) => item.asset?.rarity === shopRarityFilter)
    }

    if (shopWeaponFilter !== 'all') {
      rows = rows.filter((item) => (item.asset?.weaponKey ?? '').trim() === shopWeaponFilter)
    }

    const sorted = [...rows]
    const tieTitle = (a: ShopProductView, b: ShopProductView) => a.title.localeCompare(b.title, 'ru')

    switch (shopSort) {
      case 'price_asc':
        sorted.sort((a, b) => a.product.priceRub - b.product.priceRub || tieTitle(a, b))
        break
      case 'price_desc':
        sorted.sort((a, b) => b.product.priceRub - a.product.priceRub || tieTitle(a, b))
        break
      case 'rarity':
        sorted.sort(
          (a, b) =>
            skinRarityRank(a.asset?.rarity) - skinRarityRank(b.asset?.rarity)
            || tieTitle(a, b),
        )
        break
      case 'weapon': {
        const w = (item: ShopProductView) => (item.asset?.weaponKey ?? '').trim().toLowerCase()
        sorted.sort((a, b) => w(a).localeCompare(w(b), 'ru') || tieTitle(a, b))
        break
      }
      default:
        sorted.sort((a, b) => a.product.sortOrder - b.product.sortOrder || tieTitle(a, b))
    }

    return sorted
  }, [baseProductViews, shopSort, shopRarityFilter, shopWeaponFilter])

  const openBuyConfirmation = async (item: ShopProductView) => {
    if (item.owned || buyingProductKey || isCreatingOrder) return

    const token = getAuthToken()
    if (!token) {
      redirectToAuth(currentAppPath())
      return
    }

    setBuyingProductKey(item.product.key)
    try {
      await getCurrentUser(token)
      setDetailsProduct(null)
      setConfirmProduct(item)
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Нужно войти в аккаунт.'))
      redirectToAuth(currentAppPath())
    } finally {
      setBuyingProductKey(null)
    }
  }

  const createOrder = async () => {
    if (!confirmProduct || isCreatingOrder) return

    const token = getAuthToken()
    if (!token) {
      redirectToAuth(currentAppPath())
      return
    }

    setIsCreatingOrder(true)
    try {
      const order = await createMyShopOrder(token, {
        product_key: confirmProduct.product.key,
        locale: SHOP_LOCALE,
      })
      const checkoutUrl = order.payment?.checkoutUrl

      if (!checkoutUrl) {
        throw new Error('Платежная ссылка не была создана.')
      }

      window.location.assign(checkoutUrl)
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось создать заказ.'))
      setIsCreatingOrder(false)
    }
  }

  if (state.status === 'loading') {
    return <LoadingState title="Загружаем магазин" />
  }

  if (state.status === 'error') {
    return <ErrorState title="Магазин недоступен" message={state.error} primaryActionLabel="Повторить" onPrimaryAction={() => void load()} />
  }

  return (
    <main className="page ownership-page shop-page">
      <section className="card ownership-hero shop-hero">
        <span className="ui-badge ui-badge-accent">Магазин</span>
        <h1 className="card-title">Скины</h1>
        <p className="card-text">Выбирай скин, открывай детали и покупай сразу без корзины.</p>
      </section>

      <section className="ownership-section inventory-section">
        <div className="ownership-section-head">
          <div className="ownership-section-title">
            <h2 className="card-title">Доступно сейчас</h2>
          </div>
          <div className="ownership-section-head-actions">
            <span className="ui-badge ui-badge-neutral">{productViews.length}</span>
            {baseProductViews.length ? (
              <button
                type="button"
                className="btn btn-sm"
                aria-expanded={isShopFiltersModalOpen}
                aria-haspopup="dialog"
                onClick={() => setIsShopFiltersModalOpen(true)}
              >
                Фильтры и сортировка
              </button>
            ) : null}
          </div>
        </div>

        {productViews.length ? (
          <div className="inventory-skin-grid shop-product-grid">
            {productViews.map((item) => (
              <ProductCard
                key={item.product.id}
                item={item}
                isBuying={buyingProductKey === item.product.key}
                onOpenDetails={setDetailsProduct}
                onBuy={openBuyConfirmation}
              />
            ))}
          </div>
        ) : (
          <p className="ownership-muted inventory-empty">
            {baseProductViews.length ? 'Нет скинов по выбранным фильтрам.' : 'Скинов в магазине пока нет.'}
          </p>
        )}
      </section>

      {isShopFiltersModalOpen ? (
        <AppPortal>
          <div
            className="ui-modal-backdrop"
            role="presentation"
            onClick={() => setIsShopFiltersModalOpen(false)}
          >
            <div
              id="shop-filters-modal"
              className="ui-modal inventory-filters-modal"
              role="dialog"
              aria-modal="true"
              aria-labelledby="shop-filters-modal-title"
              onClick={(event) => event.stopPropagation()}
            >
              <div className="ui-modal-header">
                <h2 id="shop-filters-modal-title" className="ui-modal-title">Сортировка и фильтры</h2>
                <button
                  type="button"
                  className="ui-modal-close"
                  aria-label="Закрыть"
                  onClick={() => setIsShopFiltersModalOpen(false)}
                >
                  ×
                </button>
              </div>
              <div className="ui-modal-body inventory-filters-modal__body">
                <div className="inventory-toolbar inventory-toolbar--modal" aria-label="Сортировка и фильтры магазина">
                  <div className="inventory-toolbar__row">
                    <span className="inventory-toolbar__heading">Сортировка</span>
                    <fieldset className="ui-radio-group inventory-toolbar__fieldset inventory-toolbar__fieldset--inline">
                      <legend className="ui-radio-legend">Сортировка</legend>
                      {(
                        [
                          ['default', 'По умолчанию'],
                          ['price_asc', 'Цена: по возрастанию'],
                          ['price_desc', 'Цена: по убыванию'],
                          ['rarity', 'По редкости'],
                          ['weapon', 'По оружию'],
                        ] as const
                      ).map(([value, label]) => (
                        <label key={value} className="ui-radio">
                          <input
                            type="radio"
                            name="shop-sort"
                            value={value}
                            checked={shopSort === value}
                            onChange={() => setShopSort(value as ShopSortKey)}
                          />
                          <span className="ui-radio-mark" aria-hidden />
                          <span>{label}</span>
                        </label>
                      ))}
                    </fieldset>
                  </div>
                  <div className="inventory-toolbar__row inventory-toolbar__row--split">
                    <div className="inventory-toolbar__col">
                      <span className="inventory-toolbar__heading">Фильтр: редкость</span>
                      <fieldset className="ui-radio-group inventory-toolbar__fieldset">
                        <legend className="ui-radio-legend">Редкость</legend>
                        <label className="ui-radio">
                          <input
                            type="radio"
                            name="shop-filter-rarity"
                            value="all"
                            checked={shopRarityFilter === 'all'}
                            onChange={() => setShopRarityFilter('all')}
                          />
                          <span className="ui-radio-mark" aria-hidden />
                          <span>Все</span>
                        </label>
                        {(Object.keys(skinRarityConfig) as SkinRarity[]).map((rarity) => (
                          <label key={rarity} className="ui-radio">
                            <input
                              type="radio"
                              name="shop-filter-rarity"
                              value={rarity}
                              checked={shopRarityFilter === rarity}
                              onChange={() => setShopRarityFilter(rarity)}
                            />
                            <span className="ui-radio-mark" aria-hidden />
                            <span>{skinRarityConfig[rarity].label}</span>
                          </label>
                        ))}
                      </fieldset>
                    </div>
                    <div className="inventory-toolbar__col">
                      <span className="inventory-toolbar__heading">Фильтр: оружие</span>
                      <fieldset className="ui-radio-group inventory-toolbar__fieldset inventory-toolbar__fieldset--wrap">
                        <legend className="ui-radio-legend">Оружие</legend>
                        <label className="ui-radio">
                          <input
                            type="radio"
                            name="shop-filter-weapon"
                            value="all"
                            checked={shopWeaponFilter === 'all'}
                            onChange={() => setShopWeaponFilter('all')}
                          />
                          <span className="ui-radio-mark" aria-hidden />
                          <span>Все</span>
                        </label>
                        {shopWeaponOptions.map((weaponKey) => (
                          <label key={weaponKey} className="ui-radio">
                            <input
                              type="radio"
                              name="shop-filter-weapon"
                              value={weaponKey}
                              checked={shopWeaponFilter === weaponKey}
                              onChange={() => setShopWeaponFilter(weaponKey)}
                            />
                            <span className="ui-radio-mark" aria-hidden />
                            <span>{weaponKey}</span>
                          </label>
                        ))}
                      </fieldset>
                    </div>
                  </div>
                </div>
              </div>
              <div className="ui-modal-footer">
                <button type="button" className="btn primary" onClick={() => setIsShopFiltersModalOpen(false)}>
                  Готово
                </button>
              </div>
            </div>
          </div>
        </AppPortal>
      ) : null}

      {detailsProduct ? (
        <SkinDetailsModal
          item={{
            title: detailsProduct.title,
            description: detailsProduct.description,
            imageUrl: detailsProduct.imageUrl,
            accent: detailsProduct.accent,
            rarity: detailsProduct.asset?.rarity,
            weaponKey: detailsProduct.asset?.weaponKey,
            priceText: formatPrice(detailsProduct.product.priceRub),
            statusText: detailsProduct.owned ? 'Уже в инвентаре' : 'Можно купить',
          }}
          titleId="shop-product-details-title"
          onClose={() => setDetailsProduct(null)}
          footer={(
            <>
              <button className="btn" type="button" onClick={() => setDetailsProduct(null)}>Закрыть</button>
              {detailsProduct.owned ? null : (
                <button
                  className="btn primary"
                  type="button"
                  disabled={buyingProductKey === detailsProduct.product.key}
                  onClick={() => void openBuyConfirmation(detailsProduct)}
                >
                  Купить за {formatPrice(detailsProduct.product.priceRub)}
                </button>
              )}
            </>
          )}
        />
      ) : null}

      {confirmProduct ? (
        <ShopModal
          title="Подтверждение покупки"
          titleId="shop-confirm-title"
          isBusy={isCreatingOrder}
          onClose={() => !isCreatingOrder && setConfirmProduct(null)}
          footer={(
            <>
              <button className="btn" type="button" disabled={isCreatingOrder} onClick={() => setConfirmProduct(null)}>Отмена</button>
              <button className="btn primary" type="button" disabled={isCreatingOrder} onClick={() => void createOrder()}>
                {isCreatingOrder ? 'Создаем заказ...' : 'Купить'}
              </button>
            </>
          )}
        >
          <p>
            Точно купить «{confirmProduct.title}» за {formatPrice(confirmProduct.product.priceRub)}?
          </p>
        </ShopModal>
      ) : null}
    </main>
  )
}
