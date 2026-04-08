import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import toast, { Toaster } from 'react-hot-toast'
import SkinPreview2D from '../components/SkinPreview2D'
import SkinUploadInline from '../components/SkinUploadInline'
import {
  getMyInventory,
  getMyRestrictions,
  getMySquad,
  getMySquadInvites,
  getPublicAssets,
  getRestrictionMeta,
  getSquadConfig,
  getSquadMembers,
  type AssetResponse,
  type InventoryResponse,
  type RestrictionMetaResponse,
  type SquadConfigResponse,
  type SquadInviteResponse,
  type SquadMemberResponse,
  type SquadResponse,
  type UserRestrictionResponse,
} from '../services/profileHubApi'
import { buildSkinUrl } from '../services/skinApi'
import { tokenManager } from '../services/tokenManager'
import {
  ApiError,
  getCurrentUser,
  type UserResponse,
  updateNickname,
} from '../services/userApi'
import { paths } from '../routes/paths'
import { useAuthStore } from '../store/authStore'
import { useQuery } from '../util/query'
import './ProfilePage.css'

type Tab = 'overview' | 'inventory' | 'squads' | 'settings'
type Status = 'loading' | 'loaded' | 'error' | 'unauthorized'

interface DashboardData {
  user: UserResponse
  assets: AssetResponse[]
  inventory: InventoryResponse
  restrictions: UserRestrictionResponse[]
  restrictionMeta: RestrictionMetaResponse[]
  squad: SquadResponse | null
  squadMembers: SquadMemberResponse[]
  squadInvites: SquadInviteResponse[]
  squadConfig: SquadConfigResponse
}

const dtf = new Intl.DateTimeFormat('ru-RU', {
  day: '2-digit',
  month: 'short',
  year: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
})
const df = new Intl.DateTimeFormat('ru-RU', {
  day: '2-digit',
  month: 'short',
  year: 'numeric',
})

const kindLabel = (kind: string) =>
  ({
    subscription: 'Подписка',
    skin: 'Скин',
    cosmetic: 'Косметика',
    lootbox: 'Кейс',
    ticket: 'Талон',
    token: 'Токен',
    currency: 'Валюта',
  }[kind] ?? 'Предмет')

const initials = (value: string) => value.slice(0, 2).toUpperCase()
const fmtDate = (value?: string | null) => {
  if (!value) return 'Нет данных'
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : df.format(date)
}
const fmtDateTime = (value?: string | null) => {
  if (!value) return 'Нет данных'
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : dtf.format(date)
}

function mapItems(inventory: InventoryResponse, assets: AssetResponse[]) {
  const assetMap = new Map(assets.map((asset) => [asset.key, asset]))
  return [
    ...inventory.stackables.map((item) => ({
      key: item.assetKey,
      kind: assetMap.get(item.assetKey)?.assetKind ?? 'item',
      name: assetMap.get(item.assetKey)?.displayName ?? item.assetKey,
      description: assetMap.get(item.assetKey)?.description ?? null,
      model: 'stackable',
      amount: item.amount,
      updatedAt: item.updatedAt,
      expiresAt: null as string | null,
    })),
    ...inventory.entitlements.map((item) => ({
      key: item.assetKey,
      kind: assetMap.get(item.assetKey)?.assetKind ?? 'item',
      name: assetMap.get(item.assetKey)?.displayName ?? item.assetKey,
      description: assetMap.get(item.assetKey)?.description ?? null,
      model: 'entitlement',
      amount: null as number | null,
      updatedAt: item.updatedAt,
      expiresAt: null as string | null,
    })),
    ...inventory.expirables.map((item) => ({
      key: item.assetKey,
      kind: assetMap.get(item.assetKey)?.assetKind ?? 'item',
      name: assetMap.get(item.assetKey)?.displayName ?? item.assetKey,
      description: assetMap.get(item.assetKey)?.description ?? null,
      model: 'expirable',
      amount: null as number | null,
      updatedAt: item.updatedAt,
      expiresAt: item.expiresAt,
    })),
  ].sort((a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime())
}

function itemMeta(item: ReturnType<typeof mapItems>[number]) {
  if (item.model === 'stackable') return `Количество: ${item.amount ?? 0}`
  if (item.model === 'expirable') return `Активно до ${fmtDate(item.expiresAt)}`
  return 'Постоянный доступ'
}

function restrictionInfo(restriction: UserRestrictionResponse, meta: RestrictionMetaResponse[]) {
  const match = meta.find((entry) => entry.key === restriction.key)
  return {
    title: match?.locale?.en?.title ?? restriction.key,
    description:
      match?.locale?.en?.description ??
      restriction.reason ??
      'Ограничение активно для этого аккаунта.',
  }
}

function ProfileSkeletonLayout() {
  return (
    <>
      <div className="profile-tabs profile-tabs--skeleton">
        <div className="profile-shimmer profile-shimmer-tab profile-shimmer-tab--overview" />
        <div className="profile-shimmer profile-shimmer-tab profile-shimmer-tab--inventory" />
        <div className="profile-shimmer profile-shimmer-tab profile-shimmer-tab--squads" />
        <div className="profile-shimmer profile-shimmer-tab profile-shimmer-tab--settings" />
      </div>
      <div className="profile-skeleton-grid">
        <div className="card profile-skeleton-identity">
          <div className="profile-skeleton-head">
            <div className="profile-shimmer profile-shimmer-avatar" />
            <div className="profile-skeleton-head-copy">
              <div className="profile-shimmer profile-shimmer-name" />
              <div className="profile-shimmer profile-shimmer-badges" />
            </div>
          </div>
          <div className="profile-skeleton-toolbar">
            <div className="profile-shimmer profile-shimmer-kicker" />
            <div className="profile-shimmer profile-shimmer-button" />
          </div>
          <div className="profile-shimmer profile-shimmer-skin" />
          <div className="profile-skeleton-meta">
            <div className="profile-shimmer profile-shimmer-meta" />
            <div className="profile-shimmer profile-shimmer-meta" />
            <div className="profile-shimmer profile-shimmer-meta" />
          </div>
        </div>
        <div className="profile-skeleton-main">
          <div className="profile-skeleton-stats">
            {Array.from({ length: 3 }, (_, index) => (
              <div key={index} className="card profile-skeleton-stat-card">
                <div className="profile-shimmer profile-shimmer-stat-label" />
                <div className="profile-shimmer profile-shimmer-stat-value" />
                <div className="profile-shimmer profile-shimmer-stat-note" />
              </div>
            ))}
          </div>
          <div className="card profile-skeleton-panel">
            <div className="profile-skeleton-panel-head">
              <div className="profile-shimmer profile-shimmer-section" />
              <div className="profile-shimmer profile-shimmer-button profile-shimmer-button--sm" />
            </div>
            <div className="profile-shimmer profile-shimmer-item" />
            <div className="profile-shimmer profile-shimmer-item" />
            <div className="profile-shimmer profile-shimmer-item" />
          </div>
          <div className="card profile-skeleton-panel profile-skeleton-panel--summary">
            <div className="profile-skeleton-panel-head">
              <div className="profile-shimmer profile-shimmer-section" />
              <div className="profile-shimmer profile-shimmer-button profile-shimmer-button--sm" />
            </div>
            <div className="profile-shimmer profile-shimmer-summary-line" />
            <div className="profile-shimmer profile-shimmer-summary-line profile-shimmer-summary-line--short" />
            <div className="profile-skeleton-chip-row">
              <div className="profile-shimmer profile-shimmer-chip" />
              <div className="profile-shimmer profile-shimmer-chip" />
              <div className="profile-shimmer profile-shimmer-chip" />
            </div>
          </div>
        </div>
      </div>
    </>
  )
}

export default function ProfilePage() {
  const query = useQuery()
  const authHydrated = useAuthStore((store) => store.hydrated)
  const authToken = useAuthStore((store) => store.token)
  const setAuthUser = useAuthStore((store) => store.setUser)
  const setAuthToken = useAuthStore((store) => store.setToken)
  const processedUrlTokenRef = useRef<string | null>(null)

  const [status, setStatus] = useState<Status>('loading')
  const [data, setData] = useState<DashboardData | null>(null)
  const [error, setError] = useState('')
  const [activeTab, setActiveTab] = useState<Tab>('overview')
  const [nicknameDraft, setNicknameDraft] = useState('')
  const [inventorySearch, setInventorySearch] = useState('')
  const [inventoryFilter, setInventoryFilter] = useState<'all' | 'skin' | 'subscription' | 'cosmetic' | 'other'>('all')
  const [isUpdatingNickname, setIsUpdatingNickname] = useState(false)
  const [skinVersion, setSkinVersion] = useState(() => Date.now())
  const [skinFailed, setSkinFailed] = useState(false)
  const [showSkeletonOverlay, setShowSkeletonOverlay] = useState(true)
  const [contentVisible, setContentVisible] = useState(false)

  const loadDashboard = useCallback(async (token: string) => {
    try {
      const [user, inventory, assets, restrictions, squad, squadInvites, restrictionMeta, squadConfig] = await Promise.all([
        getCurrentUser(token),
        getMyInventory(token),
        getPublicAssets(),
        getMyRestrictions(token),
        getMySquad(token),
        getMySquadInvites(token),
        getRestrictionMeta(),
        getSquadConfig(),
      ])
      const squadMembers = squad ? await getSquadMembers(token, squad.id) : []
      setAuthUser({ id: user.id, username: user.username, avatarUrl: user.avatarUrl ?? '' })
      setNicknameDraft(user.username)
      setSkinFailed(false)
      setError('')
      setData({
        user,
        inventory,
        assets,
        restrictions,
        restrictionMeta,
        squad,
        squadMembers,
        squadInvites,
        squadConfig,
      })
      setStatus('loaded')
    } catch (err) {
      if (err instanceof ApiError && err.isAuthError()) {
        tokenManager.clearToken()
        setAuthToken(null)
        setAuthUser(null)
        setStatus('unauthorized')
        return
      }
      setError(err instanceof Error ? err.message : 'Не удалось загрузить профиль')
      setStatus('error')
    }
  }, [setAuthToken, setAuthUser])

  useEffect(() => {
    const requestedTab = query.get('tab')
    if (requestedTab === 'inventory' || requestedTab === 'squads' || requestedTab === 'settings') {
      setActiveTab(requestedTab)
    }
  }, [query])

  useEffect(() => {
    const run = async () => {
      if (!authHydrated) return
      const tokenFromUrl = query.get('token')
      if (tokenFromUrl) {
        if (processedUrlTokenRef.current === tokenFromUrl) return
        processedUrlTokenRef.current = tokenFromUrl
        if (!tokenManager.validateTokenFormat(tokenFromUrl)) {
          setError('Некорректный токен авторизации.')
          setStatus('error')
          return
        }
        await tokenManager.storeToken(tokenFromUrl)
        setAuthToken(tokenFromUrl)
        setStatus('loading')
        await loadDashboard(tokenFromUrl)
        const url = new URL(window.location.href)
        url.searchParams.delete('token')
        window.history.replaceState(null, '', url.pathname + url.search)
        return
      }
      processedUrlTokenRef.current = null
      if (!authToken) {
        setStatus('unauthorized')
        return
      }
      setStatus('loading')
      await loadDashboard(authToken)
    }
    void run()
  }, [authHydrated, authToken, loadDashboard, query, setAuthToken])

  const items = useMemo(() => (data ? mapItems(data.inventory, data.assets) : []), [data])
  const filteredItems = useMemo(() => items.filter((item) => {
    const queryValue = inventorySearch.trim().toLowerCase()
    const matchesSearch = !queryValue || item.name.toLowerCase().includes(queryValue) || item.key.toLowerCase().includes(queryValue)
    if (!matchesSearch) return false
    if (inventoryFilter === 'all') return true
    if (inventoryFilter === 'other') return !['skin', 'subscription', 'cosmetic'].includes(item.kind)
    return item.kind === inventoryFilter
  }), [inventoryFilter, inventorySearch, items])

  useEffect(() => {
    if (status === 'loading') {
      setShowSkeletonOverlay(true)
      if (!data) setContentVisible(false)
      return
    }
    if (status === 'loaded' && data) {
      setShowSkeletonOverlay(true)
      const frameId = window.requestAnimationFrame(() => setContentVisible(true))
      const timeoutId = window.setTimeout(() => setShowSkeletonOverlay(false), 320)
      return () => {
        window.cancelAnimationFrame(frameId)
        window.clearTimeout(timeoutId)
      }
    }
    setShowSkeletonOverlay(false)
    setContentVisible(false)
  }, [data, status])

  const onLogout = () => {
    tokenManager.clearToken()
    setAuthToken(null)
    setAuthUser(null)
    window.location.assign(paths.auth)
  }

  const onSaveNickname = async () => {
    if (!authToken || !data) return
    const nickname = nicknameDraft.trim()
    if (!nickname || nickname === data.user.username) return
    setIsUpdatingNickname(true)
    const request = updateNickname(authToken, nickname)
    toast.promise(request, {
      loading: 'Сохраняем никнейм...',
      success: 'Никнейм обновлён.',
      error: (err) => err instanceof Error ? err.message : 'Не удалось обновить никнейм',
    })
    try {
      const user = await request
      setAuthUser({ id: user.id, username: user.username, avatarUrl: user.avatarUrl ?? '' })
      setNicknameDraft(user.username)
      setData((prev) => (prev ? { ...prev, user } : prev))
    } finally {
      setIsUpdatingNickname(false)
    }
  }

  if (status === 'unauthorized') {
    return (
      <div className="ui-kit-page profile-page">
        <div className="profile-shell">
          <section className="card profile-state-card">
            <span className="ui-badge ui-badge-warning">Доступ</span>
            <h1 className="card-title">Нужно войти</h1>
            <p className="card-text">Профиль, инвентарь и сквад доступны после входа через Discord.</p>
            <button className="btn primary" type="button" onClick={() => window.location.assign(paths.auth)}>
              Войти
            </button>
          </section>
        </div>
      </div>
    )
  }

  if (status === 'error' || (!data && status !== 'loading')) {
    return (
      <div className="ui-kit-page profile-page">
        <div className="profile-shell">
          <section className="card profile-state-card">
            <span className="ui-badge ui-badge-warning">Ошибка</span>
            <h1 className="card-title">Профиль недоступен</h1>
            <p className="card-text">{error || 'Не удалось загрузить данные аккаунта.'}</p>
            <div className="profile-actions">
              <button className="btn primary" type="button" onClick={() => window.location.reload()}>
                Повторить
              </button>
              <button className="btn" type="button" onClick={onLogout}>Сбросить сессию</button>
            </div>
          </section>
        </div>
      </div>
    )
  }

  const summary = data
    ? { inventory: items.length }
    : { inventory: 0 }

  return (
    <div className="ui-kit-page profile-page">
      <Toaster
        position="top-right"
        toastOptions={{
          duration: 3000,
          style: {
            background: '#19191c',
            color: '#fff',
            border: '1px solid rgba(255,255,255,.08)',
            borderRadius: '0',
          },
        }}
      />
      <div className="profile-shell">
        <div className="profile-transition-shell">
          {data ? <div className={`profile-content-stage ${contentVisible ? 'is-visible' : ''}`}>
            <div className="profile-topbar">
              <nav className="profile-tabs" aria-label="Разделы профиля">
                {([
                  ['overview', 'Обзор', null],
                  ['inventory', 'Инвентарь', summary.inventory],
                  ['squads', 'Сквады', null],
                  ['settings', 'Настройки', null],
                ] as const).map(([key, label, count]) => (
                  <button key={key} className={`profile-tab ${activeTab === key ? 'is-active' : ''}`} type="button" onClick={() => setActiveTab(key)}>
                    <span>{label}</span>
                    {typeof count === 'number' ? <span className="profile-tab-count">{count}</span> : null}
                  </button>
                ))}
              </nav>
            </div>

            {activeTab === 'overview' ? (
              <div className="profile-layout">
                <aside className="card profile-identity">
                  <div className="profile-identity-head">
                    {data.user.avatarUrl ? <img className="profile-avatar" src={data.user.avatarUrl} alt={data.user.username} /> : <div className="ui-avatar profile-avatar-fallback">{initials(data.user.username)}</div>}
                    <div>
                      <h1 className="profile-name">{data.user.username}</h1>
                      <div className="profile-chip-row">
                        {data.user.isSuperuser ? <span className="ui-badge ui-badge-secondary">Админ</span> : null}
                        <span className="ui-badge ui-badge-neutral">{data.user.isActive ? 'Активен' : 'Отключён'}</span>
                        {data.squad ? <span className="ui-badge ui-badge-neutral">В скваде</span> : null}
                      </div>
                    </div>
                  </div>

                  <div className="profile-skin-card">
                    <div className="profile-section-head">
                      <span className="ui-section-title">Текущий скин</span>
                      <button className="btn btn-sm" type="button" onClick={() => setActiveTab('settings')}>Изменить</button>
                    </div>
                    <div className="profile-skin-preview">
                      {!skinFailed ? <SkinPreview2D className="profile-skin-render" src={buildSkinUrl(data.user.id, skinVersion)} alt={`Скин ${data.user.username}`} onError={() => setSkinFailed(true)} /> : <div className="profile-empty profile-empty--centered"><span className="ui-badge ui-badge-neutral">Нет скина</span><p>Загрузите PNG в настройках профиля.</p></div>}
                    </div>
                  </div>

                  <dl className="profile-kv">
                    <div><dt>Последний вход</dt><dd>{fmtDateTime(data.user.lastLoginAt)}</dd></div>
                    <div><dt>Создан</dt><dd>{fmtDate(data.user.createdAt)}</dd></div>
                    <div><dt>Discord ID</dt><dd>{data.user.discordId}</dd></div>
                  </dl>
                </aside>

                <div className="profile-main">
                  <section className="profile-stats">
                    {[
                      ['Инвентарь', String(items.length), 'предметов и доступов'],
                      ['Сквад', data.squad ? 'Есть' : 'Нет', data.squad ? 'команда подключена к аккаунту' : 'вы не состоите в скваде'],
                      ['Ограничения', String(data.restrictions.length), data.restrictions.length ? 'активные ограничения' : 'доступ без блокировок'],
                    ].map(([label, value, note]) => (
                      <article key={label} className="card profile-stat-card">
                        <span className="profile-stat-label">{label}</span>
                        <strong className="profile-stat-value">{value}</strong>
                        <span className="profile-stat-note">{note}</span>
                      </article>
                    ))}
                  </section>

                  <section className="card profile-panel">
                    <div className="ui-card-header">
                      <h2 className="card-title">Последние предметы</h2>
                      <button className="btn btn-sm" type="button" onClick={() => setActiveTab('inventory')}>Открыть инвентарь</button>
                    </div>
                    {items.length ? <div className="profile-list">{items.slice(0, 6).map((item) => <article key={`${item.key}:${item.updatedAt}`} className="profile-list-item"><div><div className="profile-item-title"><strong>{item.name}</strong><span className="ui-badge ui-badge-secondary">{kindLabel(item.kind)}</span></div><span className="profile-subtle">{itemMeta(item)}</span></div></article>)}</div> : <div className="profile-empty"><span className="ui-badge ui-badge-neutral">Пусто</span><p>Когда в аккаунте появятся предметы, они будут показаны здесь.</p></div>}
                  </section>

                  <section className="profile-summary-grid profile-summary-grid--single">
                    <article className="card profile-panel">
                      <div className="ui-card-header">
                        <h2 className="card-title">Сквад</h2>
                        <button className="btn btn-sm" type="button" onClick={() => setActiveTab('squads')}>Открыть</button>
                      </div>
                      {data.squad ? <div className="profile-stack"><strong>{data.squad.name}</strong><span className="profile-subtle">{data.squad.memberCount} / {data.squad.maxMembers} участников</span><div className="profile-chip-row">{data.squadMembers.slice(0, 5).map((member) => <span key={member.id} className="ui-chip"><span className="ui-chip-label">{member.username}{member.isLeader ? ' • лидер' : ''}</span></span>)}</div></div> : <div className="profile-empty"><span className="ui-badge ui-badge-neutral">Нет сквада</span><p>Состав команды и приглашения появятся здесь автоматически.</p></div>}
                    </article>
                  </section>
                </div>
              </div>
            ) : null}

            {activeTab === 'inventory' ? (
              <section className="card profile-panel">
                <div className="profile-toolbar">
                  <label className="ui-search">
                    <span className="ui-search-icon" aria-hidden>?</span>
                    <input className="ui-search-input" type="search" value={inventorySearch} onChange={(e) => setInventorySearch(e.target.value)} placeholder="Поиск по имени или ключу" />
                  </label>
                  <div className="profile-filter-row">
                    {([['all', 'Все'], ['skin', 'Скины'], ['subscription', 'Подписки'], ['cosmetic', 'Косметика'], ['other', 'Прочее']] as const).map(([key, label]) => (
                      <button key={key} className={`profile-tab profile-tab--small ${inventoryFilter === key ? 'is-active' : ''}`} type="button" onClick={() => setInventoryFilter(key)}>{label}</button>
                    ))}
                  </div>
                </div>
                {filteredItems.length ? <div className="profile-inventory-grid">{filteredItems.map((item) => <article key={`${item.key}:${item.updatedAt}`} className="profile-inventory-card"><div className="profile-item-title"><span className="ui-badge ui-badge-secondary">{kindLabel(item.kind)}</span><span className="profile-subtle">{item.model}</span></div><h3>{item.name}</h3><p>{item.description ?? 'Описание пока не задано в каталоге.'}</p><div className="profile-inventory-footer"><span>{itemMeta(item)}</span><code>{item.key}</code></div></article>)}</div> : <div className="profile-empty profile-empty--wide"><span className="ui-badge ui-badge-neutral">Ничего не найдено</span><p>Измените поиск или фильтр. Если инвентарь пуст, предметы появятся здесь позже.</p></div>}
              </section>
            ) : null}

            {activeTab === 'squads' ? (
              <div className="profile-split">
                <section className="card profile-panel">
                  <div className="ui-card-header">
                    <h2 className="card-title">Мой сквад</h2>
                  </div>
                  {data.squad ? <div className="profile-stack"><strong>{data.squad.name}</strong><span className="profile-subtle">Создан {fmtDate(data.squad.createdAt)}</span>{data.squad.isRestricted ? <div className="ui-alert ui-alert-warning"><span className="ui-alert-icon" aria-hidden>!</span><span>{data.squad.restrictionReason ?? 'Для сквада действуют ограничения.'}</span></div> : null}<div className="profile-member-list">{data.squadMembers.map((member) => <div key={member.id} className="profile-member"><div className="profile-member-avatar">{member.avatarUrl ? <img src={member.avatarUrl} alt={member.username} /> : <span>{initials(member.username)}</span>}</div><div><strong>{member.username}</strong><span className="profile-subtle">{member.isLeader ? 'Лидер' : 'Участник'}</span></div></div>)}</div></div> : <div className="profile-empty profile-empty--wide"><span className="ui-badge ui-badge-neutral">Нет сквада</span><p>Когда у аккаунта появится сквад, блок заполнится автоматически.</p></div>}
                </section>

                <section className="card profile-panel">
                  <div className="ui-card-header">
                    <h2 className="card-title">Приглашения и ограничения</h2>
                    <span className="ui-badge ui-badge-warning">{data.squadInvites.length} приглашений</span>
                  </div>
                  {data.squadInvites.length ? <div className="profile-stack">{data.squadInvites.map((invite) => <div key={invite.id} className="profile-inline-card"><strong>{invite.squadName}</strong><span className="profile-subtle">Истекает {fmtDateTime(invite.expiresAt)}</span></div>)}</div> : <div className="profile-empty"><span className="ui-badge ui-badge-neutral">Нет инвайтов</span><p>Новые приглашения в сквад будут показаны здесь.</p></div>}
                  <div className="ui-divider" />
                  {data.restrictions.length ? <div className="profile-stack">{data.restrictions.map((restriction) => { const info = restrictionInfo(restriction, data.restrictionMeta); return <div key={`${restriction.key}:${restriction.createdAt}`} className="ui-alert ui-alert-warning profile-alert"><span className="ui-alert-icon" aria-hidden>!</span><div><strong>{info.title}</strong><span>{info.description}</span></div></div> })}</div> : <div className="profile-empty"><span className="ui-badge ui-badge-success">Без ограничений</span><p>Создание и вступление в сквад доступны без дополнительных блокировок.</p></div>}
                </section>
              </div>
            ) : null}

            {activeTab === 'settings' ? (
              <div className="profile-split">
                <section className="card profile-panel">
                  <div className="ui-card-header"><h2 className="card-title">Данные аккаунта</h2></div>
                  <div className="profile-form">
                    <div className="ui-field">
                      <label className="ui-label" htmlFor="profile-nickname">Никнейм</label>
                      <input id="profile-nickname" className="ui-input" value={nicknameDraft} onChange={(e) => setNicknameDraft(e.target.value)} maxLength={16} placeholder="Введите никнейм" disabled={isUpdatingNickname} />
                      <div className="ui-hint">Изменение сохранится в учётной записи после подтверждения.</div>
                    </div>
                    <div className="profile-actions">
                      <button className="btn primary" type="button" disabled={isUpdatingNickname} onClick={() => void onSaveNickname()}>{isUpdatingNickname ? 'Сохранение...' : 'Сохранить никнейм'}</button>
                    </div>
                  </div>
                  <dl className="profile-kv profile-kv--wide">
                    <div><dt>E-mail</dt><dd>{data.user.email ?? 'Не указан'}</dd></div>
                    <div><dt>Discord</dt><dd>{data.user.discordId}</dd></div>
                    <div><dt>Последний вход</dt><dd>{fmtDateTime(data.user.lastLoginAt)}</dd></div>
                  </dl>
                </section>

                <section className="card profile-panel">
                  <div className="ui-card-header"><h2 className="card-title">Скин и сессия</h2></div>
                  <div className="profile-stack">
                    <div className="profile-skin-preview profile-skin-preview--large">
                      {!skinFailed ? <SkinPreview2D className="profile-skin-render" src={buildSkinUrl(data.user.id, skinVersion)} alt={`Скин ${data.user.username}`} onError={() => setSkinFailed(true)} /> : <div className="profile-empty profile-empty--centered"><span className="ui-badge ui-badge-neutral">Нет скина</span><p>Загрузите PNG-файл, чтобы добавить внешний вид персонажа.</p></div>}
                    </div>
                    <SkinUploadInline onUploaded={() => setSkinVersion(Date.now())} />
                    <button className="btn danger" type="button" onClick={onLogout}>Выйти из аккаунта</button>
                  </div>
                </section>
              </div>
            ) : null}
          </div> : null}

          {showSkeletonOverlay ? <div className={`profile-loading-overlay ${status === 'loaded' && contentVisible ? 'is-exiting' : ''}`} aria-hidden><ProfileSkeletonLayout /></div> : null}
        </div>
      </div>
    </div>
  )
}

