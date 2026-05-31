import { useEffect, useMemo, useState, type CSSProperties, type ReactNode } from 'react'
import toast from 'react-hot-toast'
import { toDisplayError } from '../api/http'
import {
  buildPublicAssetImageUrl,
  listAllPublicAssets,
  listMyEntitlements,
  type AssetResponse,
  type EntitlementResponse,
  type SkinRarity,
} from '../api/inventory'
import {
  getPublicLootbox,
  listPublicLootboxes,
  type LootboxDefinitionResponse,
  type LootboxDetailResponse,
  type LootboxDropResponse,
} from '../api/lootboxes'
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
const DEFAULT_STACKABLE_MAX_PURCHASE_QUANTITY = 99

type ShopSortKey = 'default' | 'price_asc' | 'price_desc' | 'rarity' | 'weapon'
type ShopRarityFilter = 'all' | 'none' | SkinRarity
type ShopWeaponFilter = 'all' | string
type ShopAssetKindFilter = 'all' | string

type ShopState =
  | { status: 'loading' }
  | { status: 'error'; error: string }
  | { status: 'ready'; products: ShopProductResponse[]; assets: AssetResponse[]; entitlements: EntitlementResponse[]; lootboxes: LootboxDefinitionResponse[] }

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

function maxPurchaseQuantity(item: ShopProductView | null) {
  if (!item) return 1
  if (normalizedOwnershipModel(item.product, item.asset) !== 'stackable') return 1
  return item.product.maxPerPurchase ?? DEFAULT_STACKABLE_MAX_PURCHASE_QUANTITY
}

function clampPurchaseQuantity(item: ShopProductView | null, value: number) {
  const max = maxPurchaseQuantity(item)
  if (!Number.isFinite(value)) return 1
  return Math.max(1, Math.min(max, Math.trunc(value)))
}

function formatDuration(seconds: number | null | undefined) {
  if (!seconds || seconds <= 0) return null

  const days = Math.floor(seconds / 86400)
  const hours = Math.floor((seconds % 86400) / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)

  const parts: string[] = []
  if (days) parts.push(`${days} дн.`)
  if (hours) parts.push(`${hours} ч.`)
  if (!days && minutes) parts.push(`${minutes} мин.`)

  return parts.length ? parts.join(' ') : 'меньше минуты'
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
  return rarity ? skinRarityConfig[rarity].label : 'Товар'
}

function assetKindLabel(assetKind: string | null | undefined) {
  switch ((assetKind ?? '').trim().toLowerCase()) {
    case 'skin':
      return 'Скин'
    case 'subscription':
      return 'Подписка'
    case 'kit':
      return 'Кит'
    case 'lootbox':
      return 'Лутбокс'
    case 'currency':
      return 'Валюта'
    case 'cosmetic':
      return 'Косметика'
    case 'ticket':
      return 'Билет'
    case 'token':
      return 'Токен'
    case 'item':
      return 'Предмет'
    default:
      return assetKind?.trim() || 'Товар'
  }
}

function normalizedAssetKind(product: ShopProductResponse, asset: AssetResponse | null) {
  return (
    metadataString(product, ['assetKind', 'kind', 'type'])
    ?? metadataString(asset, ['assetKind', 'kind', 'type'])
    ?? asset?.assetKind
    ?? 'item'
  ).trim().toLowerCase()
}

function normalizedOwnershipModel(product: ShopProductResponse, asset: AssetResponse | null) {
  return (asset?.ownershipModel ?? product.ownershipModel).trim().toLowerCase()
}

function isSkinProduct(product: ShopProductResponse, asset: AssetResponse | null) {
  return normalizedAssetKind(product, asset) === 'skin' || Boolean(asset?.weaponKey || asset?.rarity)
}

function productCardLabel(product: ShopProductResponse, asset: AssetResponse | null) {
  if (isSkinProduct(product, asset) && asset?.rarity) {
    return rarityLabel(asset.rarity)
  }

  return assetKindLabel(normalizedAssetKind(product, asset))
}

function uniqueSortedValues(values: Array<string | null | undefined>, labelForValue: (value: string) => string) {
  return Array.from(
    new Set(
      values
        .map((value) => value?.trim().toLowerCase() ?? '')
        .filter(Boolean),
    ),
  ).sort((a, b) => labelForValue(a).localeCompare(labelForValue(b), 'ru'))
}

function buildAssetMap(assets: AssetResponse[]) {
  const map = new Map<string, AssetResponse>()
  for (const asset of assets) {
    map.set(asset.key, asset)
    map.set(asset.id, asset)
  }
  return map
}

function buildLootboxMap(lootboxes: LootboxDefinitionResponse[]) {
  const map = new Map<string, LootboxDefinitionResponse>()
  for (const lootbox of lootboxes) {
    map.set(lootbox.assetKey, lootbox)
    map.set(lootbox.assetDefinitionId, lootbox)
    map.set(lootbox.id, lootbox)
  }
  return map
}

function productRewardView(assetMap: Map<string, AssetResponse>, assetKey: string, assetDefinitionId: string) {
  const asset = assetMap.get(assetKey) ?? assetMap.get(assetDefinitionId) ?? null
  return {
    title: asset?.displayName ?? assetKey,
    imageUrl: assetImageUrl(asset),
    accent: asset?.rarity
      ? skinRarityConfig[asset.rarity].color
      : metadataString(asset, ['accentColor', 'color', 'rarityColor']) ?? fallbackAccent(assetKey),
  }
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
    ?? 'Товар для твоего инвентаря.'
}

function displaySkinTitle(title: string, asset: AssetResponse | null) {
  if (!asset?.weaponKey || !title.includes('|')) return title

  return title.split('|').slice(1).join('|').trim() || title
}

function addMetaItem(rows: Array<{ label: string; value: string }>, label: string, value: string | null | undefined) {
  if (!value) return
  rows.push({ label, value })
}

function buildProductMetaItems(product: ShopProductResponse, asset: AssetResponse | null) {
  const rows: Array<{ label: string; value: string }> = []
  const assetKind = normalizedAssetKind(product, asset)
  const ownershipModel = normalizedOwnershipModel(product, asset)
  const isSkin = isSkinProduct(product, asset)
  const isSubscription = assetKind === 'subscription'

  addMetaItem(rows, 'Цена', formatPrice(product.priceRub))
  addMetaItem(rows, 'Тип', assetKindLabel(assetKind))

  if (isSkin) {
    if (asset?.rarity) addMetaItem(rows, 'Редкость', rarityLabel(asset.rarity))
  }

  if (!isSkin && (isSubscription || ownershipModel === 'expirable')) {
    addMetaItem(rows, 'Длительность', formatDuration(product.durationSeconds))
  }

  if (ownershipModel === 'stackable') {
    addMetaItem(rows, 'Количество за покупку', product.stackableAmount ? String(product.stackableAmount) : null)
    addMetaItem(rows, 'Максимум за покупку', product.maxPerPurchase ? String(product.maxPerPurchase) : null)
  }

  return rows
}

type ShopProductView = {
  product: ShopProductResponse
  asset: AssetResponse | null
  lootbox: LootboxDefinitionResponse | null
  title: string
  description: string
  imageUrl: string | null
  modelPreview: {
    modelUrl: string
    textureUrl: string
  } | null
  accent: string
  owned: boolean
}

type ShopLootboxDetailsState =
  | { status: 'loading'; item: ShopProductView }
  | { status: 'ready'; item: ShopProductView; detail: LootboxDetailResponse }
  | { status: 'error'; item: ShopProductView; error: string }

function resolveModelPreviewUrl(value: string | null) {
  if (!value) return null
  if (value.startsWith('/') || value.startsWith('http://') || value.startsWith('https://')) return value
  return null
}

function productModelPreview(product: ShopProductResponse, asset: AssetResponse | null) {
  const rawModel =
    asset?.modelUrl
    ?? metadataString(asset, ['modelUrl', 'model_url', 'geckoModelUrl', 'gecko_model_url', 'model'])
    ?? metadataString(product, ['modelUrl', 'model_url', 'geckoModelUrl', 'gecko_model_url', 'model'])
    ?? null
  const rawTexture =
    asset?.textureUrl
    ?? metadataString(asset, ['textureUrl', 'texture_url', 'geckoTextureUrl', 'gecko_texture_url', 'texture'])
    ?? metadataString(product, ['textureUrl', 'texture_url', 'geckoTextureUrl', 'gecko_texture_url', 'texture'])
    ?? null

  const modelUrl = resolveModelPreviewUrl(rawModel)
  const textureUrl = resolveModelPreviewUrl(rawTexture)
  return modelUrl && textureUrl ? { modelUrl, textureUrl } : null
}

function InventoryVisual({
  item,
  compact = false,
  fallbackLabel,
}: {
  item: { title: string; imageUrl: string | null; accent: string }
  compact?: boolean
  fallbackLabel?: string
}) {
  const [failedImageUrl, setFailedImageUrl] = useState<string | null>(null)
  const fallback = fallbackLabel ?? (item.title.trim().slice(0, 1).toUpperCase() || 'S')
  const showImage = Boolean(item.imageUrl) && failedImageUrl !== item.imageUrl

  return (
    <div className={`inventory-visual ${compact ? 'inventory-visual--compact' : ''}`} style={cardStyle(item.accent)}>
      {showImage ? <img src={item.imageUrl ?? ''} alt="" loading="lazy" onError={() => setFailedImageUrl(item.imageUrl)} /> : <span>{fallback}</span>}
      <div className="inventory-visual-splash" />
    </div>
  )
}

function formatDropAmount(drop: LootboxDropResponse) {
  if (drop.amount !== null && drop.amount !== undefined) return `${drop.amount} шт`
  if (drop.durationSeconds !== null && drop.durationSeconds !== undefined) {
    const days = Math.floor(drop.durationSeconds / 86_400)
    if (days > 0) return `${days} д.`

    const hours = Math.max(1, Math.floor(drop.durationSeconds / 3_600))
    return `${hours} ч.`
  }
  return '1 шт'
}

function formatDropDetails(drop: LootboxDropResponse) {
  const rows = [formatDropAmount(drop)]
  if (drop.duplicateCompensationAmount !== null && drop.duplicateCompensationAmount !== undefined) {
    rows.push(`компенсация дубля: ${drop.duplicateCompensationAmount}`)
  }
  return rows.join(' · ')
}

function formatDropChance(drop: LootboxDropResponse) {
  if (drop.totalWeight <= 0) return null
  const chance = drop.weight / drop.totalWeight * 100
  return `${chance >= 10 ? chance.toFixed(0) : chance.toFixed(1)}%`
}

function ShopLootboxDetailsModal({
  state,
  assetMap,
  isBuying,
  onBuy,
  onClose,
}: {
  state: ShopLootboxDetailsState
  assetMap: Map<string, AssetResponse>
  isBuying: boolean
  onBuy: (item: ShopProductView) => void
  onClose: () => void
}) {
  const { item } = state
  const drops = state.status === 'ready'
    ? state.detail.drops.filter((drop) => drop.isActive).sort((left, right) => left.sortOrder - right.sortOrder)
    : []

  return (
    <AppPortal>
      <div className="ui-modal-backdrop" role="presentation" onClick={onClose}>
        <div className="ui-modal skin-details-modal lootbox-details-modal" role="dialog" aria-modal="true" aria-labelledby="shop-lootbox-details-title" onClick={(event) => event.stopPropagation()}>
          <div className="ui-modal-header">
            <h2 id="shop-lootbox-details-title" className="ui-modal-title">{item.title}</h2>
            <button className="ui-modal-close" type="button" aria-label="Закрыть" onClick={onClose}>
              ×
            </button>
          </div>
          <div className="ui-modal-body">
            <div className="skin-details-modal__body">
              <InventoryVisual item={item} fallbackLabel="Кейс" />
              <div className="skin-details-modal__content">
                <p>{item.description || 'Описание кейса пока не заполнено.'}</p>
                <dl className="skin-details-modal__meta">
                  {buildProductMetaItems(item.product, item.asset).map((metaItem) => (
                    <div key={`${metaItem.label}:${metaItem.value}`}>
                      <dt>{metaItem.label}</dt>
                      <dd>{metaItem.value}</dd>
                    </div>
                  ))}
                </dl>

                <section className="lootbox-details-modal__drops" aria-label="Содержимое кейса">
                  <h3>Может выпасть</h3>
                  {state.status === 'loading' ? (
                    <p className="ownership-muted">Загружаем содержимое...</p>
                  ) : null}
                  {state.status === 'error' ? (
                    <p className="ownership-muted">{state.error}</p>
                  ) : null}
                  {state.status === 'ready' && drops.length === 0 ? (
                    <p className="ownership-muted">Активные награды не указаны.</p>
                  ) : null}
                  {drops.length ? (
                    <div className="lootbox-details-modal__drop-list">
                      {drops.map((drop) => {
                        const reward = productRewardView(assetMap, drop.rewardAssetKey, drop.rewardAssetDefinitionId)
                        const chance = formatDropChance(drop)

                        return (
                          <article key={drop.id} className="lootbox-details-modal__drop" style={cardStyle(reward.accent)}>
                            <InventoryVisual item={reward} compact />
                            <div className="lootbox-details-modal__drop-main">
                              <strong>{drop.rewardAssetDisplayName || reward.title}</strong>
                              <span>{formatDropDetails(drop)}</span>
                            </div>
                            {chance ? <span className="lootbox-details-modal__drop-chance">{chance}</span> : null}
                          </article>
                        )
                      })}
                    </div>
                  ) : null}
                </section>
              </div>
            </div>
          </div>
          <div className="ui-modal-footer">
            <button className="btn" type="button" onClick={onClose}>Закрыть</button>
            {item.owned ? null : (
              <button
                className="btn primary"
                type="button"
                disabled={isBuying}
                onClick={() => onBuy(item)}
              >
                Купить за {formatPrice(item.product.priceRub)}
              </button>
            )}
          </div>
        </div>
      </div>
    </AppPortal>
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
  const assetKind = normalizedAssetKind(item.product, item.asset)

  return (
    <article
      className={`inventory-skin-card shop-product-card ${item.owned ? 'is-owned' : ''}`}
      data-rarity={item.asset?.rarity ?? 'none'}
      data-asset-kind={assetKind}
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
          <span>{productCardLabel(item.product, item.asset)}</span>
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
  const [detailsLootbox, setDetailsLootbox] = useState<ShopLootboxDetailsState | null>(null)
  const [confirmProduct, setConfirmProduct] = useState<ShopProductView | null>(null)
  const [confirmQuantity, setConfirmQuantity] = useState(1)
  const [buyingProductKey, setBuyingProductKey] = useState<string | null>(null)
  const [isCreatingOrder, setIsCreatingOrder] = useState(false)
  const [shopSort, setShopSort] = useState<ShopSortKey>('default')
  const [shopRarityFilter, setShopRarityFilter] = useState<ShopRarityFilter>('all')
  const [shopWeaponFilter, setShopWeaponFilter] = useState<ShopWeaponFilter>('all')
  const [shopAssetKindFilter, setShopAssetKindFilter] = useState<ShopAssetKindFilter>('all')
  const [isShopFiltersModalOpen, setIsShopFiltersModalOpen] = useState(false)

  const load = async () => {
    setState({ status: 'loading' })
    try {
      const token = getAuthToken()
      const [products, assets, entitlements, lootboxes] = await Promise.all([
        listPublicShopProducts(SHOP_LOCALE),
        listAllPublicAssets(),
        token ? listMyEntitlements(token).catch(() => [] as EntitlementResponse[]) : Promise.resolve([] as EntitlementResponse[]),
        listPublicLootboxes().catch(() => [] as LootboxDefinitionResponse[]),
      ])

      setState({ status: 'ready', products, assets, entitlements, lootboxes })
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
    const lootboxMap = buildLootboxMap(state.lootboxes)
    const ownedSet = buildOwnedSet(state.entitlements)

    return state.products
      .map((product) => {
        const asset = assetMap.get(product.assetKey) ?? assetMap.get(product.assetDefinitionId) ?? null
        const lootbox = lootboxMap.get(product.assetKey) ?? lootboxMap.get(product.assetDefinitionId) ?? null
        const title = displaySkinTitle(product.localizedName || asset?.displayName || product.assetDisplayName || product.key, asset)
        const description = productDescription(product, asset)

        return {
          product,
          asset,
          lootbox,
          title,
          description,
          imageUrl: assetImageUrl(asset),
          modelPreview: productModelPreview(product, asset),
          accent: productAccent(product, asset),
          owned: isOwned(product, ownedSet),
        }
      })
  }, [state])

  const assetMap = useMemo(
    () => state.status === 'ready' ? buildAssetMap(state.assets) : new Map<string, AssetResponse>(),
    [state],
  )

  const shopWeaponOptions = useMemo(
    () => uniqueSortedWeaponKeys(baseProductViews.map((item) => item.asset?.weaponKey)),
    [baseProductViews],
  )

  const shopAssetKindOptions = useMemo(
    () => uniqueSortedValues(
      baseProductViews.map((item) => normalizedAssetKind(item.product, item.asset)),
      assetKindLabel,
    ),
    [baseProductViews],
  )

  const productViews = useMemo(() => {
    let rows = baseProductViews

    if (shopAssetKindFilter !== 'all') {
      rows = rows.filter((item) => normalizedAssetKind(item.product, item.asset) === shopAssetKindFilter)
    }

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
  }, [baseProductViews, shopSort, shopRarityFilter, shopWeaponFilter, shopAssetKindFilter])

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
      setDetailsLootbox(null)
      setConfirmQuantity(1)
      setConfirmProduct(item)
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Нужно войти в аккаунт.'))
      redirectToAuth(currentAppPath())
    } finally {
      setBuyingProductKey(null)
    }
  }

  const openProductDetails = async (item: ShopProductView) => {
    if (normalizedAssetKind(item.product, item.asset) !== 'lootbox' || !item.lootbox) {
      setDetailsProduct(item)
      return
    }

    setDetailsProduct(null)
    setDetailsLootbox({ status: 'loading', item })
    try {
      const detail = await getPublicLootbox(item.lootbox.id)
      setDetailsLootbox({ status: 'ready', item, detail })
    } catch (cause) {
      setDetailsLootbox({
        status: 'error',
        item,
        error: toDisplayError(cause, 'Не удалось загрузить содержимое кейса.'),
      })
    }
  }

  const createOrder = async () => {
    if (!confirmProduct || isCreatingOrder) return
    const quantity = clampPurchaseQuantity(confirmProduct, confirmQuantity)

    const token = getAuthToken()
    if (!token) {
      redirectToAuth(currentAppPath())
      return
    }

    setIsCreatingOrder(true)
    try {
      const order = await createMyShopOrder(token, {
        product_key: confirmProduct.product.key,
        quantity,
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

  const confirmMaxQuantity = maxPurchaseQuantity(confirmProduct)
  const normalizedConfirmQuantity = clampPurchaseQuantity(confirmProduct, confirmQuantity)
  const isConfirmQuantityEditable = confirmMaxQuantity > 1
  const confirmTotalPrice = confirmProduct ? confirmProduct.product.priceRub * normalizedConfirmQuantity : 0
  const confirmQuantityPresets = [1, 3, 5, 10]

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
        <h1 className="card-title">Товары</h1>
        <p className="card-text">Выбирай товар, открывай детали и покупай сразу без корзины.</p>
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
                onOpenDetails={(selected) => void openProductDetails(selected)}
                onBuy={openBuyConfirmation}
              />
            ))}
          </div>
        ) : (
          <p className="ownership-muted inventory-empty">
            {baseProductViews.length ? 'Нет товаров по выбранным фильтрам.' : 'Товаров в магазине пока нет.'}
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
                      <fieldset className="ui-radio-group inventory-toolbar__fieldset inventory-toolbar__fieldset--wrap">
                        <legend className="ui-radio-legend">Тип товара</legend>
                        <label className="ui-radio">
                          <input
                            type="radio"
                            name="shop-filter-kind"
                            value="all"
                            checked={shopAssetKindFilter === 'all'}
                            onChange={() => setShopAssetKindFilter('all')}
                          />
                          <span className="ui-radio-mark" aria-hidden />
                          <span>Все</span>
                        </label>
                        {shopAssetKindOptions.map((assetKind) => (
                          <label key={assetKind} className="ui-radio">
                            <input
                              type="radio"
                              name="shop-filter-kind"
                              value={assetKind}
                              checked={shopAssetKindFilter === assetKind}
                              onChange={() => setShopAssetKindFilter(assetKind)}
                            />
                            <span className="ui-radio-mark" aria-hidden />
                            <span>{assetKindLabel(assetKind)}</span>
                          </label>
                        ))}
                      </fieldset>
                    </div>
                  </div>
                  <div className="inventory-toolbar__row inventory-toolbar__row--split">
                    <div className="inventory-toolbar__col">
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
                        <label className="ui-radio">
                          <input
                            type="radio"
                            name="shop-filter-rarity"
                            value="none"
                            checked={shopRarityFilter === 'none'}
                            onChange={() => setShopRarityFilter('none')}
                          />
                          <span className="ui-radio-mark" aria-hidden />
                          <span>Без редкости</span>
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
            metaItems: buildProductMetaItems(detailsProduct.product, detailsProduct.asset),
            modelPreview: detailsProduct.modelPreview,
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

      {detailsLootbox ? (
        <ShopLootboxDetailsModal
          state={detailsLootbox}
          assetMap={assetMap}
          isBuying={buyingProductKey === detailsLootbox.item.product.key}
          onBuy={(selected) => void openBuyConfirmation(selected)}
          onClose={() => setDetailsLootbox(null)}
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
          <div className="shop-confirm">
            <p className="shop-confirm__title">
              Купить «{confirmProduct.title}»
            </p>
            {isConfirmQuantityEditable ? (
              <div className="shop-confirm__quantity">
                <span id="shop-confirm-quantity-label">Количество</span>
                <div className="shop-confirm__quantity-row">
                  <div className="shop-confirm__quantity-control">
                    <button
                      className="btn btn-sm"
                      type="button"
                      disabled={isCreatingOrder || normalizedConfirmQuantity <= 1}
                      onClick={() => setConfirmQuantity((value) => clampPurchaseQuantity(confirmProduct, value - 1))}
                    >
                      -
                    </button>
                    <input
                      className="ui-input"
                      type="number"
                      min={1}
                      max={confirmMaxQuantity}
                      value={normalizedConfirmQuantity}
                      disabled={isCreatingOrder}
                      aria-labelledby="shop-confirm-quantity-label"
                      onChange={(event) => setConfirmQuantity(clampPurchaseQuantity(confirmProduct, Number(event.target.value)))}
                    />
                    <button
                      className="btn btn-sm"
                      type="button"
                      disabled={isCreatingOrder || normalizedConfirmQuantity >= confirmMaxQuantity}
                      onClick={() => setConfirmQuantity((value) => clampPurchaseQuantity(confirmProduct, value + 1))}
                    >
                      +
                    </button>
                  </div>
                  <div className="shop-confirm__presets" aria-label="Быстрый выбор количества">
                    {confirmQuantityPresets.map((preset) => (
                      <button
                        key={preset}
                        className={`shop-confirm__preset${normalizedConfirmQuantity === preset ? ' is-active' : ''}`}
                        type="button"
                        disabled={isCreatingOrder || preset > confirmMaxQuantity}
                        onClick={() => setConfirmQuantity(clampPurchaseQuantity(confirmProduct, preset))}
                      >
                        x{preset}
                      </button>
                    ))}
                  </div>
                </div>
              </div>
            ) : null}
            <dl className="shop-confirm__summary">
              <div>
                <dt>Цена за штуку</dt>
                <dd>{formatPrice(confirmProduct.product.priceRub)}</dd>
              </div>
              <div>
                <dt>Количество</dt>
                <dd>{normalizedConfirmQuantity}</dd>
              </div>
              <div>
                <dt>Итого</dt>
                <dd>{formatPrice(confirmTotalPrice)}</dd>
              </div>
            </dl>
          </div>
        </ShopModal>
      ) : null}
    </main>
  )
}
