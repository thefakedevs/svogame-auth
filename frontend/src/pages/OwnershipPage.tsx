import { useEffect, useMemo, useState } from 'react'
import {
  getMyInventory,
  listAllPublicAssets,
  type AssetResponse,
  type EntitlementResponse,
  type ExpirableResponse,
  type InventoryResponse,
  type StackableResponse,
} from '../api/ownership'
import { toDisplayError } from '../api/http'
import ErrorState from '../components/ErrorState'
import LoadingState from '../components/LoadingState'
import { currentAppPath, redirectToAuth } from '../routes/auth'
import { getAuthToken } from '../shared/session/auth-session'
import './OwnershipPage.css'

type OwnershipState =
  | { status: 'loading' }
  | { status: 'unauthorized' }
  | { status: 'error'; error: string }
  | { status: 'ready'; inventory: InventoryResponse; assets: AssetResponse[] }

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

function assetTitle(assetMap: Map<string, AssetResponse>, assetKey: string, assetDefinitionId: string) {
  return assetMap.get(assetKey)?.displayName ?? assetMap.get(assetDefinitionId)?.displayName ?? assetKey
}

function assetDescription(assetMap: Map<string, AssetResponse>, assetKey: string, assetDefinitionId: string) {
  const asset = assetMap.get(assetKey) ?? assetMap.get(assetDefinitionId)
  return asset?.description ?? asset?.assetKind ?? assetKey
}

function makeAssetMap(assets: AssetResponse[]) {
  const map = new Map<string, AssetResponse>()
  for (const asset of assets) {
    map.set(asset.key, asset)
    map.set(asset.id, asset)
  }
  return map
}

function EmptySection({ text }: { text: string }) {
  return <p className="ownership-muted">{text}</p>
}

function StackableItem({ item, assetMap }: { item: StackableResponse; assetMap: Map<string, AssetResponse> }) {
  return (
    <article className="ownership-item">
      <div className="ownership-item-head">
        <div className="ownership-item-title">
          <strong>{assetTitle(assetMap, item.assetKey, item.assetDefinitionId)}</strong>
          <small>{assetDescription(assetMap, item.assetKey, item.assetDefinitionId)}</small>
        </div>
        <span className="ui-badge ui-badge-neutral">{item.assetKey}</span>
      </div>
      <div className="ownership-item-meta">
        <span>Количество: {item.amount}</span>
        <span>Обновлено: {formatDateTime(item.updatedAt)}</span>
      </div>
    </article>
  )
}

function EntitlementItem({ item, assetMap }: { item: EntitlementResponse; assetMap: Map<string, AssetResponse> }) {
  return (
    <article className="ownership-item">
      <div className="ownership-item-head">
        <div className="ownership-item-title">
          <strong>{assetTitle(assetMap, item.assetKey, item.assetDefinitionId)}</strong>
          <small>{assetDescription(assetMap, item.assetKey, item.assetDefinitionId)}</small>
        </div>
        <span className="ui-badge ui-badge-success">Есть</span>
      </div>
      <div className="ownership-item-meta">
        <span>Выдано: {formatDateTime(item.grantedAt)}</span>
        <span>Обновлено: {formatDateTime(item.updatedAt)}</span>
      </div>
    </article>
  )
}

function ExpirableItem({ item, assetMap }: { item: ExpirableResponse; assetMap: Map<string, AssetResponse> }) {
  return (
    <article className="ownership-item">
      <div className="ownership-item-head">
        <div className="ownership-item-title">
          <strong>{assetTitle(assetMap, item.assetKey, item.assetDefinitionId)}</strong>
          <small>{assetDescription(assetMap, item.assetKey, item.assetDefinitionId)}</small>
        </div>
        <span className={`ui-badge ${item.isActive ? 'ui-badge-success' : 'ui-badge-warning'}`}>
          {item.isActive ? 'Активно' : 'Истекло'}
        </span>
      </div>
      <div className="ownership-item-meta">
        <span>До: {formatDateTime(item.expiresAt)}</span>
        <span>Выдано: {formatDateTime(item.grantedAt)}</span>
      </div>
    </article>
  )
}

export default function OwnershipPage() {
  const [state, setState] = useState<OwnershipState>({ status: 'loading' })

  const load = async () => {
    const token = getAuthToken()
    if (!token) {
      setState({ status: 'unauthorized' })
      return
    }

    setState({ status: 'loading' })
    try {
      const [inventory, assets] = await Promise.all([
        getMyInventory(token),
        listAllPublicAssets().catch(() => [] as AssetResponse[]),
      ])
      setState({ status: 'ready', inventory, assets })
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

  if (state.status === 'loading') {
    return <LoadingState title="Загружаем инвентарь" />
  }

  if (state.status === 'unauthorized') {
    return (
      <section className="card profile-state-card">
        <span className="ui-badge ui-badge-warning">Доступ</span>
        <h1 className="card-title">Нужно войти</h1>
        <p className="card-text">Инвентарь доступно только после входа через Discord.</p>
        <button className="btn primary" type="button" onClick={() => redirectToAuth(currentAppPath())}>
          Войти
        </button>
      </section>
    )
  }

  if (state.status === 'error') {
    return <ErrorState title="Инвентарь недоступно" message={state.error} primaryActionLabel="Повторить" onPrimaryAction={() => void load()} />
  }

  const { inventory } = state
  const total = inventory.stackables.length + inventory.entitlements.length + inventory.expirables.length

  return (
    <main className="page ownership-page">
      <section className="card ownership-hero">
        <span className="ui-badge ui-badge-secondary">Ownership</span>
        <h1 className="card-title">Инвентарь</h1>
        <p className="card-text">Все невалютные ассеты аккаунта: накапливаемые предметы, постоянные права и временные владения.</p>
      </section>

      <section className="ownership-summary" aria-label="Сводка имущества">
        <article className="ownership-summary-card">
          <span>Всего ассетов</span>
          <strong>{total}</strong>
        </article>
        <article className="ownership-summary-card">
          <span>Права доступа</span>
          <strong>{inventory.entitlements.length}</strong>
        </article>
        <article className="ownership-summary-card">
          <span>Временные</span>
          <strong>{inventory.expirables.length}</strong>
          <small>{inventory.expirables.filter((item) => item.isActive).length} активных</small>
        </article>
      </section>

      <section className="card ownership-grid">
        <div className="ownership-section">
          <div className="ownership-section-head">
            <h2 className="card-title">Stackable</h2>
            <span className="ui-badge ui-badge-neutral">{inventory.stackables.length}</span>
          </div>
          {inventory.stackables.length ? (
            <div className="ownership-items">
              {inventory.stackables.map((item) => <StackableItem key={item.assetKey} item={item} assetMap={assetMap} />)}
            </div>
          ) : <EmptySection text="Накапливаемых предметов пока нет." />}
        </div>

        <div className="ownership-section">
          <div className="ownership-section-head">
            <h2 className="card-title">Entitlement</h2>
            <span className="ui-badge ui-badge-neutral">{inventory.entitlements.length}</span>
          </div>
          {inventory.entitlements.length ? (
            <div className="ownership-items">
              {inventory.entitlements.map((item) => <EntitlementItem key={item.assetKey} item={item} assetMap={assetMap} />)}
            </div>
          ) : <EmptySection text="Постоянных прав доступа пока нет." />}
        </div>

        <div className="ownership-section">
          <div className="ownership-section-head">
            <h2 className="card-title">Expirable</h2>
            <span className="ui-badge ui-badge-neutral">{inventory.expirables.length}</span>
          </div>
          {inventory.expirables.length ? (
            <div className="ownership-items">
              {inventory.expirables.map((item) => <ExpirableItem key={item.assetKey} item={item} assetMap={assetMap} />)}
            </div>
          ) : <EmptySection text="Временных владений пока нет." />}
        </div>
      </section>
    </main>
  )
}
