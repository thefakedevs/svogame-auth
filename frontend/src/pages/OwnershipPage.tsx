import { useEffect, useMemo, useState, type CSSProperties } from 'react'
import toast from 'react-hot-toast'
import {
  buildPublicAssetImageUrl,
  getMyInventory,
  listMyGunskinSelections,
  listAllPublicAssets,
  resetMyGunskin,
  selectMyGunskin,
  type AssetResponse,
  type EntitlementResponse,
  type InventoryResponse,
  type SelectedGunskinResponse,
  type SkinRarity,
  type StackableResponse,
} from '../api/inventory'
import { toDisplayError } from '../api/http'
import ErrorState from '../components/ErrorState'
import LoadingState from '../components/LoadingState'
import SkinDetailsModal from '../components/SkinDetailsModal'
import { currentAppPath, redirectToAuth } from '../routes/auth'
import { getAuthToken } from '../shared/session/auth-session'
import './OwnershipPage.css'

type OwnershipState =
  | { status: 'loading' }
  | { status: 'unauthorized' }
  | { status: 'error'; error: string }
  | { status: 'ready'; inventory: InventoryResponse; assets: AssetResponse[]; selectedGunskins: SelectedGunskinResponse[] }

const defaultSubscriptions = [
  {
    assetKey: 'subscription_plus',
    title: 'Subscription Plus',
    description: 'Базовая подписка аккаунта.',
    accent: '#2db7a3',
  },
  {
    assetKey: 'subscription_pro',
    title: 'Subscription Pro',
    description: 'Расширенная подписка аккаунта.',
    accent: '#f7a41d',
  },
] as const

const skinRarityConfig: Record<SkinRarity, { label: string; color: string }> = {
  common: { label: 'Обычный', color: '#9aa8b5' },
  rare: { label: 'Редкий', color: '#2f8cff' },
  legendary: { label: 'Легендарный', color: '#ffb02e' },
}

const dateTimeFormatter = new Intl.DateTimeFormat('ru-RU', {
  day: '2-digit',
  month: 'short',
  year: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
})

function formatDateTime(value?: string | null) {
  if (!value) return 'Нет данных'
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : dateTimeFormatter.format(date).replace(",", " ")
}

function makeAssetMap(assets: AssetResponse[]) {
  const map = new Map<string, AssetResponse>()
  for (const asset of assets) {
    map.set(asset.key, asset)
    map.set(asset.id, asset)
  }
  return map
}

function getAsset(assetMap: Map<string, AssetResponse>, assetKey: string, assetDefinitionId: string) {
  return assetMap.get(assetKey) ?? assetMap.get(assetDefinitionId) ?? null
}

function metadataRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === 'object' && !Array.isArray(value) ? value as Record<string, unknown> : null
}

function metadataString(asset: AssetResponse | null, keys: string[]) {
  const metadata = metadataRecord(asset?.metadata)
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

function assetAccent(asset: AssetResponse | null, assetKey: string) {
  return asset?.rarity
    ? skinRarityConfig[asset.rarity].color
    : metadataString(asset, ['accentColor', 'color', 'rarityColor']) ?? fallbackAccent(asset?.key ?? assetKey)
}

function assetView(assetMap: Map<string, AssetResponse>, assetKey: string, assetDefinitionId: string) {
  const asset = getAsset(assetMap, assetKey, assetDefinitionId)
  return {
    asset,
    key: assetKey,
    title: asset?.displayName ?? assetKey,
    description: asset?.description ?? asset?.assetKind ?? assetKey,
    imageUrl: assetImageUrl(asset),
    accent: assetAccent(asset, assetKey),
    rarity: asset?.rarity ?? null,
    weaponKey: asset?.weaponKey ?? null,
  }
}

function isSkinAsset(asset: AssetResponse | null, assetKey: string) {
  const haystack = [
    assetKey,
    asset?.key,
    asset?.displayName,
    asset?.assetKind,
    asset?.weaponKey,
    metadataString(asset, ['type', 'category', 'kind', 'itemType']),
  ].filter(Boolean).join(' ').toLowerCase()

  return haystack.includes('skin') || haystack.includes('скин')
}

function rarityLabel(rarity: SkinRarity | null) {
  return rarity ? skinRarityConfig[rarity].label : 'Без редкости'
}

function skinRarityAccent(rarity: SkinRarity | null) {
  return rarity ? skinRarityConfig[rarity].color : '#8fa3ad'
}

function isSelectedSkin(item: SkinCardItem, selected: SelectedGunskinResponse | null | undefined) {
  return Boolean(selected && (
    selected.assetKey === item.assetKey
    || selected.assetDefinitionId === item.assetDefinitionId
  ))
}

function subscriptionTier(asset: AssetResponse | null, assetKey: string, title: string) {
  const source = `${asset?.key ?? assetKey} ${title}`.toLowerCase()
  if (source.includes('pro')) return 'PRO'
  if (source.includes('plus')) return 'PLUS'
  return 'Подписка'
}

function remainingText(value: string | null | undefined, isActive: boolean) {
  if (!isActive) return 'Не активна'
  if (!value) return 'Активна'

  const expiresAt = new Date(value).getTime()
  if (Number.isNaN(expiresAt)) return 'Активна'

  const diff = expiresAt - Date.now()
  if (diff <= 0) return 'Истекает'

  const days = Math.floor(diff / 86_400_000)
  if (days > 0) return `Осталось ${days} д.`

  const hours = Math.max(1, Math.floor(diff / 3_600_000))
  return `Осталось ${hours} ч.`
}

function cardStyle(accent: string): CSSProperties {
  return { '--inventory-accent': accent } as CSSProperties
}

type InventoryVisualItem = {
  title: string
  imageUrl: string | null
  accent: string
}

function InventoryVisual({ item, compact = false }: { item: InventoryVisualItem; compact?: boolean }) {
  const [failedImageUrl, setFailedImageUrl] = useState<string | null>(null)
  const fallback = item.title.trim().slice(0, 1).toUpperCase() || 'I'
  const showImage = Boolean(item.imageUrl) && failedImageUrl !== item.imageUrl

  return (
    <div className={`inventory-visual ${compact ? 'inventory-visual--compact' : ''}`} style={cardStyle(item.accent)}>
      {showImage ? <img src={item.imageUrl ?? ''} alt="" loading="lazy" onError={() => setFailedImageUrl(item.imageUrl)} /> : <span>{fallback}</span>}
      {/* splash for better image visibility */}
      <div className="inventory-visual-splash" />
    </div>
  )
}

function EmptySection({ text }: { text: string }) {
  return <p className="ownership-muted inventory-empty">{text}</p>
}

function SectionTitle({ title, count, showCount = true }: { title: string; count: number; showCount?: boolean }) {
  return (
    <div className="ownership-section-head">
      <div className="ownership-section-title">
        <h2 className="card-title">{title}</h2>
      </div>
      {showCount ? <span className="ui-badge ui-badge-neutral">{count}</span> : null}
    </div>
  )
}

type InventoryAssetView = ReturnType<typeof assetView>

type SkinCardItem = InventoryAssetView & {
  assetKey: string
  assetDefinitionId: string
  amount?: number
  grantedAt?: string
  updatedAt?: string
}

function SkinCard({
  item,
  isSelected,
  isSelecting,
  isResetting,
  onOpenDetails,
  onSelect,
  onReset,
}: {
  item: SkinCardItem
  isSelected: boolean
  isSelecting: boolean
  isResetting: boolean
  onOpenDetails: (item: SkinCardItem) => void
  onSelect: (item: SkinCardItem) => void
  onReset: (item: SkinCardItem) => void
}) {
  const accent = skinRarityAccent(item.rarity)
  const canSelect = Boolean(item.weaponKey)
  const isBusy = isSelecting || isResetting
  const [isActionHovered, setIsActionHovered] = useState(false)
  const actionLabel = isResetting
    ? 'Убираем...'
    : isSelecting
      ? 'Выбираем...'
      : isSelected
        ? isActionHovered ? 'Убрать выбор' : 'Выбрано'
        : canSelect ? 'Выбрать' : 'Нет оружия'

  return (
    <article
      className={`inventory-skin-card ${isSelected ? 'is-selected' : ''} ${isBusy ? 'is-selecting' : ''}`}
      data-rarity={item.rarity ?? 'none'}
      style={cardStyle(accent)}
    >
      <button className="inventory-skin-card__preview" type="button" onClick={() => onOpenDetails(item)}>
        <InventoryVisual item={{ ...item, accent }} />
      </button>
      <div className="inventory-skin-card__body">
        <div className="inventory-card-title">
          <strong>{item.title}</strong>
        </div>
        <div className="inventory-skin-card__rarity">
          <span>{rarityLabel(item.rarity)}</span>
        </div>
        <div className="inventory-skin-card__selection">
          <button
            className={`inventory-skin-card__action ${isSelected ? 'is-selected' : ''} ${isSelected && isActionHovered ? 'is-remove-intent' : ''}`}
            type="button"
            disabled={!canSelect || isBusy}
            onMouseEnter={() => setIsActionHovered(true)}
            onMouseLeave={() => setIsActionHovered(false)}
            onFocus={() => setIsActionHovered(true)}
            onBlur={() => setIsActionHovered(false)}
            onClick={() => {
              if (isSelected) {
                onReset(item)
                return
              }
              onSelect(item)
            }}
          >
            {actionLabel}
          </button>
        </div>
        <div className="inventory-card-meta">
          <span>
            {item.amount !== undefined
              ? `Обновлено: ${formatDateTime(item.updatedAt)}`
              : `Выдано: ${formatDateTime(item.grantedAt)}`}
          </span>
        </div>
      </div>
    </article>
  )
}

function ItemCard({ item }: { item: InventoryAssetView & StackableResponse }) {
  return (
    <article className="inventory-compact-card" style={cardStyle(item.accent)}>
      <InventoryVisual item={item} compact />
      <div className="inventory-card-title">
        <strong>{item.title}</strong>
        <small>{item.description}</small>
      </div>
      <div className="inventory-amount">
        <strong>{item.amount}</strong>
        <span>шт.</span>
      </div>
    </article>
  )
}

function AccessCard({ item }: { item: InventoryAssetView & EntitlementResponse }) {
  return (
    <article className="inventory-access-card" style={cardStyle(item.accent)}>
      <InventoryVisual item={item} compact />
      <div className="inventory-card-title">
        <strong>{item.title}</strong>
        <small>{item.description}</small>
      </div>
      <div className="inventory-access-state">
        <span className="ui-badge ui-badge-success">Открыт</span>
        <small>с {formatDateTime(item.grantedAt)}</small>
      </div>
    </article>
  )
}

type SubscriptionCardItem = InventoryAssetView & {
  assetKey: string
  assetDefinitionId: string
  expiresAt?: string | null
  grantedAt?: string | null
  updatedAt?: string | null
  isActive: boolean
  lastExtendedAt?: string | null
}

function SubscriptionCard({ item }: { item: SubscriptionCardItem }) {
  const tier = subscriptionTier(item.asset, item.assetKey, item.title)

  return (
    <article className={`inventory-subscription-card ${item.isActive ? 'is-active' : 'is-expired'}`} style={cardStyle(item.accent)}>
      <div className="inventory-subscription-card__top">
        <span className="inventory-subscription-tier">{tier}</span>
        <span className={`ui-badge ${item.isActive ? 'ui-badge-success' : 'ui-badge-warning'}`}>
          {item.isActive ? 'Активна' : 'Неактивна'}
        </span>
      </div>
      <div className="inventory-card-title">
        <strong>{item.title}</strong>
        <small>{item.description}</small>
      </div>
      <div className="inventory-subscription-status">
        <strong>{remainingText(item.expiresAt, item.isActive)}</strong>
        <span>{item.isActive && item.expiresAt ? `до ${formatDateTime(item.expiresAt)}` : 'Подписка не подключена'}</span>
      </div>
    </article>
  )
}

void ItemCard
void AccessCard
void SubscriptionCard

export default function OwnershipPage() {
  const [state, setState] = useState<OwnershipState>({ status: 'loading' })
  const [selectingSkinKey, setSelectingSkinKey] = useState<string | null>(null)
  const [resettingSkinKey, setResettingSkinKey] = useState<string | null>(null)
  const [detailsSkin, setDetailsSkin] = useState<SkinCardItem | null>(null)

  const load = async () => {
    const token = getAuthToken()
    if (!token) {
      setState({ status: 'unauthorized' })
      return
    }

    setState({ status: 'loading' })
    try {
      const [inventory, assets, selectedGunskinItems] = await Promise.all([
        getMyInventory(token),
        listAllPublicAssets().catch(() => [] as AssetResponse[]),
        listMyGunskinSelections(token),
      ])
      const selectedGunskins = selectedGunskinItems.map((item) => item.selected).filter(Boolean)
      setState({ status: 'ready', inventory, assets, selectedGunskins })
    } catch (cause) {
      setState({ status: 'error', error: toDisplayError(cause, 'Не удалось загрузить инвентарь.') })
    }
  }

  useEffect(() => {
    queueMicrotask(() => void load())
  }, [])

  const assetMap = useMemo(() => {
    if (state.status !== 'ready') return new Map<string, AssetResponse>()
    return makeAssetMap(state.assets)
  }, [state])

  const selectedGunskinByWeapon = useMemo(() => {
    const map = new Map<string, SelectedGunskinResponse>()
    if (state.status !== 'ready') return map

    for (const selected of state.selectedGunskins) {
      map.set(selected.weaponKey, selected)
    }
    return map
  }, [state])

  if (state.status === 'loading') {
    return <LoadingState title="Загружаем инвентарь" />
  }

  if (state.status === 'unauthorized') {
    return (
      <section className="card profile-state-card">
        <span className="ui-badge ui-badge-warning">Доступ</span>
        <h1 className="card-title">Нужно войти</h1>
        <p className="card-text">Инвентарь доступен только после входа через Discord.</p>
        <button className="btn primary" type="button" onClick={() => redirectToAuth(currentAppPath())}>
          Войти
        </button>
      </section>
    )
  }

  if (state.status === 'error') {
    return <ErrorState title="Инвентарь недоступен" message={state.error} primaryActionLabel="Повторить" onPrimaryAction={() => void load()} />
  }

  const { inventory } = state
  const stackableEntries = inventory.stackables.map((item) => ({ ...item, ...assetView(assetMap, item.assetKey, item.assetDefinitionId) }))
  const entitlementEntries = inventory.entitlements.map((item) => ({ ...item, ...assetView(assetMap, item.assetKey, item.assetDefinitionId) }))
  const skinEntries: SkinCardItem[] = [
    ...stackableEntries
      .filter((item) => isSkinAsset(item.asset, item.assetKey))
      .map((item) => ({ ...item, amount: item.amount, updatedAt: item.updatedAt })),
    ...entitlementEntries
      .filter((item) => isSkinAsset(item.asset, item.assetKey))
      .map((item) => ({ ...item, grantedAt: item.grantedAt, updatedAt: item.updatedAt })),
  ]
  const itemEntries = stackableEntries.filter((item) => !isSkinAsset(item.asset, item.assetKey))
  const ownedSubscriptionEntries = inventory.expirables.map((item) => ({ ...item, ...assetView(assetMap, item.assetKey, item.assetDefinitionId) }))
  const ownedSubscriptionByKey = new Map(ownedSubscriptionEntries.map((item) => [item.assetKey, item]))
  const defaultSubscriptionKeys = new Set<string>(defaultSubscriptions.map((item) => item.assetKey))
  const subscriptionEntries: SubscriptionCardItem[] = [
    ...defaultSubscriptions.map((fallback) => {
      const owned = ownedSubscriptionByKey.get(fallback.assetKey)
      if (owned) return owned

      const view = assetView(assetMap, fallback.assetKey, fallback.assetKey)
      return {
        ...view,
        title: view.asset ? view.title : fallback.title,
        description: view.asset ? view.description : fallback.description,
        accent: view.asset ? view.accent : fallback.accent,
        assetKey: fallback.assetKey,
        assetDefinitionId: view.asset?.id ?? fallback.assetKey,
        expiresAt: null,
        grantedAt: null,
        updatedAt: null,
        isActive: false,
        lastExtendedAt: null,
      }
    }),
    ...ownedSubscriptionEntries.filter((item) => !defaultSubscriptionKeys.has(item.assetKey)),
  ]
  void itemEntries
  void subscriptionEntries
  const selectSkin = async (item: SkinCardItem) => {
    const token = getAuthToken()
    if (!token) {
      setState({ status: 'unauthorized' })
      return
    }
    if (!item.weaponKey || selectingSkinKey) return

    const requestKey = `${item.weaponKey}:${item.assetKey}`
    setSelectingSkinKey(requestKey)
    try {
      const selected = await selectMyGunskin(token, item.weaponKey, {
        assetKey: item.assetKey,
        reasonCode: 'inventory_page',
        reasonText: 'Selected from inventory page',
      })

      setState((prev) => {
        if (prev.status !== 'ready') return prev

        const nextSelections = prev.selectedGunskins.filter((entry) => entry.weaponKey !== selected.weaponKey)
        nextSelections.push(selected)

        return {
          ...prev,
          selectedGunskins: nextSelections,
        }
      })
      toast.success('Скин выбран активным.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось выбрать скин.'))
    } finally {
      setSelectingSkinKey(null)
    }
  }

  const resetSkin = async (item: SkinCardItem) => {
    const token = getAuthToken()
    if (!token) {
      setState({ status: 'unauthorized' })
      return
    }
    if (!item.weaponKey || selectingSkinKey || resettingSkinKey) return

    const requestKey = `${item.weaponKey}:${item.assetKey}`
    setResettingSkinKey(requestKey)
    try {
      await resetMyGunskin(token, item.weaponKey)

      setState((prev) => {
        if (prev.status !== 'ready') return prev

        return {
          ...prev,
          selectedGunskins: prev.selectedGunskins.filter((entry) => entry.weaponKey !== item.weaponKey),
        }
      })
      toast.success('Выбор скина снят.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось снять выбранный скин.'))
    } finally {
      setResettingSkinKey(null)
    }
  }

  return (
    <main className="page ownership-page">
      <section className="card ownership-hero">
        <h1 className="card-title">Инвентарь</h1>
        <p className="card-text">Скины, предметы и подписки аккаунта собраны по типам, чтобы быстро найти нужное даже в большом инвентаре.</p>
      </section>

      <section className="ownership-section inventory-section">
        <SectionTitle
          title="Скины"
          count={skinEntries.length}
        />
        {skinEntries.length ? (
          <div className="inventory-skin-grid">
            {skinEntries.map((item) => {
              const selected = item.weaponKey ? selectedGunskinByWeapon.get(item.weaponKey) : null
              const requestKey = item.weaponKey ? `${item.weaponKey}:${item.assetKey}` : null

              return (
                <SkinCard
                  key={`${item.key}-${item.assetDefinitionId}`}
                  item={item}
                  isSelected={isSelectedSkin(item, selected)}
                  isSelecting={requestKey !== null && requestKey === selectingSkinKey}
                  isResetting={requestKey !== null && requestKey === resettingSkinKey}
                  onOpenDetails={setDetailsSkin}
                  onSelect={selectSkin}
                  onReset={resetSkin}
                />
              )
            })}
          </div>
        ) : <EmptySection text="Скинов в инвентаре пока нет." />}
      </section>

      {detailsSkin ? (
        <SkinDetailsModal
          item={{
            title: detailsSkin.title,
            description: detailsSkin.description,
            imageUrl: detailsSkin.imageUrl,
            accent: skinRarityAccent(detailsSkin.rarity),
            rarity: detailsSkin.rarity,
            weaponKey: detailsSkin.weaponKey,
            statusText: 'Уже в инвентаре',
          }}
          titleId="inventory-skin-details-title"
          onClose={() => setDetailsSkin(null)}
          footer={(
            <button className="btn" type="button" onClick={() => setDetailsSkin(null)}>Закрыть</button>
          )}
        />
      ) : null}
{/* 
      <section className="ownership-section inventory-section">
        <SectionTitle
          title="Предметы"
          count={itemEntries.length}
        />
        {itemEntries.length ? (
          <div className="inventory-compact-grid">
            {itemEntries.map((item) => <ItemCard key={item.assetKey} item={item} />)}
          </div>
        ) : <EmptySection text="Предметов с количеством пока нет." />}
      </section>

      <section className="ownership-section inventory-section">
        <SectionTitle
          title="Подписки"
          count={subscriptionEntries.length}
          showCount={false}
        />
        <div className="inventory-subscription-grid">
          {subscriptionEntries.map((item) => <SubscriptionCard key={item.assetKey} item={item} />)}
        </div>
      </section> */}
    </main>
  )
}
