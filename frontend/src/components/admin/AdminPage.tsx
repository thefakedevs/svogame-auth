import { useEffect, useMemo, useRef, useState } from 'react'
import toast from 'react-hot-toast'
import { ApiError, toDisplayError } from '../../api/http'
import {
  createAdminServiceToken,
  deleteAdminSquad,
  deleteAdminUserSkin,
  deleteAdminSquadImage,
  getAdminDefaultSkin,
  getAdminMe,
  getAdminServiceTokenAudit,
  getAdminSquad,
  getAdminUser,
  getAdminUserSquad,
  getPublicSquadMembers,
  grantAdminUserSuperuser,
  listAdminSquads,
  listAdminServiceTokens,
  listAdminUsers,
  patchAdminSquad,
  revokeAdminUserSuperuser,
  restrictAdminSquad,
  revokeAdminServiceToken,
  rotateAdminServiceToken,
  unrestrictAdminSquad,
  uploadAdminDefaultSkin,
  type AdminSquadResponse,
  type AdminUserResponse,
  type DefaultSkinResponse,
  type ServiceTokenAuditResponse,
  type ServiceTokenResponse,
} from '../../api/admin'
import type { SquadMemberResponse } from '../../api/squads'
import { buildSkinUrl, type SkinModel } from '../../api/skins'
import { adminSquadPath, adminTokenAuditPath, adminUserPath, paths } from '../../routes/paths'
import { pushUrl, replaceUrl, usePathname } from '../../shared/navigation/history'
import { getAuthToken } from '../../shared/session/auth-session'
import { useQuery } from '../../util/query'
import AdminAssetsPanel from './AdminAssetsPanel'
import AdminGunskinShopWizard from './AdminGunskinShopWizard'
import AdminShopPanel from './AdminShopPanel'
import AdminShopProductView from './AdminShopProductView'
import AdminSquadProfile from './AdminSquadProfile'
import AdminUserProfile from './AdminUserProfile'
import ErrorState from '../ErrorState'
import LoadingState from '../LoadingState'
import './AdminPage.css'

type AdminRoute =
  | { type: 'home' }
  | { type: 'user'; userId: string }
  | { type: 'squad'; squadId: string }
  | { type: 'tokenAudit'; tokenId: string }
  | { type: 'shopProduct'; productId: string }
type AccessState = 'loading' | 'allowed' | 'denied' | 'error'
type HomeTab = 'overview' | 'users' | 'squads' | 'tokens' | 'assets' | 'shop' | 'skinShop'
const PAGE_SIZE = 30

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
  return Number.isNaN(date.getTime()) ? value : dateTimeFormatter.format(date).replace(', ', ' ')
}

function initials(value: string) {
  return value.slice(0, 2).toUpperCase()
}

function formatMetadata(value: unknown) {
  if (value == null) return '—'
  try {
    const text = JSON.stringify(value)
    return text.length > 180 ? `${text.slice(0, 180)}...` : text
  } catch {
    return String(value)
  }
}

function appendVersion(url: string, version: number) {
  return `${url}${url.includes('?') ? '&' : '?'}v=${version}`
}

function parseRoute(pathname: string): AdminRoute {
  const userMatch = pathname.match(/^\/admin\/users\/([^/]+)$/)
  if (userMatch) return { type: 'user', userId: decodeURIComponent(userMatch[1]) }

  const squadMatch = pathname.match(/^\/admin\/squads\/([^/]+)$/)
  if (squadMatch) return { type: 'squad', squadId: decodeURIComponent(squadMatch[1]) }

  const auditMatch = pathname.match(/^\/admin\/tokens\/([^/]+)\/audit$/)
  if (auditMatch) return { type: 'tokenAudit', tokenId: decodeURIComponent(auditMatch[1]) }

  const shopProductMatch = pathname.match(/^\/admin\/shop\/products\/([^/]+)$/)
  if (shopProductMatch) return { type: 'shopProduct', productId: decodeURIComponent(shopProductMatch[1]) }

  return { type: 'home' }
}

function tabFromQuery(tab: string | null): HomeTab {
  if (tab === 'overview') return 'overview'
  if (tab === 'squads') return 'squads'
  if (tab === 'tokens') return 'tokens'
  if (tab === 'assets') return 'assets'
  if (tab === 'shop') return 'shop'
  if (tab === 'skinShop') return 'skinShop'
  if (tab === 'users') return 'users'
  return 'overview'
}

function setHomeTab(tab: HomeTab) {
  if (typeof window === 'undefined') return
  const url = new URL(window.location.href)
  url.searchParams.set('tab', tab)
  replaceUrl(url.pathname + url.search)
}

export default function AdminPage() {
  const token = getAuthToken()
  const pathname = usePathname() || paths.admin
  const query = useQuery()
  const route = useMemo(() => parseRoute(pathname), [pathname])
  const tab = tabFromQuery(query.get('tab'))

  const [accessState, setAccessState] = useState<AccessState>('loading')
  const [error, setError] = useState('')

  const [users, setUsers] = useState<AdminUserResponse[]>([])
  const [squads, setSquads] = useState<AdminSquadResponse[]>([])
  const [tokens, setTokens] = useState<ServiceTokenResponse[]>([])
  const [usersQuery, setUsersQuery] = useState('')
  const [squadsQuery, setSquadsQuery] = useState('')
  const [usersPage, setUsersPage] = useState(1)
  const [squadsPage, setSquadsPage] = useState(1)
  const [usersHasMore, setUsersHasMore] = useState(false)
  const [squadsHasMore, setSquadsHasMore] = useState(false)
  const [isUsersLoadingMore, setIsUsersLoadingMore] = useState(false)
  const [isSquadsLoadingMore, setIsSquadsLoadingMore] = useState(false)

  const [user, setUser] = useState<AdminUserResponse | null>(null)
  const [userSquad, setUserSquad] = useState<AdminSquadResponse | null | undefined>(undefined)

  const [squad, setSquad] = useState<AdminSquadResponse | null>(null)
  const [squadMembers, setSquadMembers] = useState<SquadMemberResponse[]>([])

  useEffect(() => {
    const run = async () => {
      if (!token) {
        setAccessState('denied')
        return
      }

      setAccessState('loading')
      try {
        const me = await getAdminMe(token)
        setAccessState(me.isSuperuser ? 'allowed' : 'denied')
      } catch (cause) {
        if (cause instanceof ApiError && cause.isAuthError()) {
          setAccessState('denied')
          return
        }
        setError(toDisplayError(cause, 'Не удалось проверить доступ к админке.'))
        setAccessState('error')
      }
    }
    void run()
  }, [token])

  useEffect(() => {
    if (!token || accessState !== 'allowed' || route.type !== 'home') return

    if (tab === 'users') {
      void listAdminUsers(token, { q: usersQuery.trim() || undefined, page: 1, perPage: PAGE_SIZE })
        .then((response) => {
          setUsers(response.items)
          setUsersPage(1)
          setUsersHasMore(response.page < response.totalPages && response.items.length < response.total)
        })
        .catch((cause) => setError(toDisplayError(cause, 'Ошибка загрузки пользователей.')))
    }

    if (tab === 'squads') {
      void listAdminSquads(token, { q: squadsQuery.trim() || undefined, page: 1, perPage: PAGE_SIZE })
        .then((response) => {
          setSquads(response.items)
          setSquadsPage(1)
          setSquadsHasMore(response.page * response.perPage < response.total)
        })
        .catch((cause) => setError(toDisplayError(cause, 'Ошибка загрузки сквадов.')))
    }

    if (tab === 'tokens') {
      void listAdminServiceTokens(token)
        .then(setTokens)
        .catch((cause) => setError(toDisplayError(cause, 'Ошибка загрузки сервисных токенов.')))
    }
  }, [accessState, route.type, tab, token, usersQuery, squadsQuery])

  const loadMoreUsers = async () => {
    if (!token || !usersHasMore || isUsersLoadingMore) return
    setIsUsersLoadingMore(true)
    try {
      const nextPage = usersPage + 1
      const response = await listAdminUsers(token, {
        q: usersQuery.trim() || undefined,
        page: nextPage,
        perPage: PAGE_SIZE,
      })
      setUsers((prev) => [...prev, ...response.items])
      setUsersPage(nextPage)
      setUsersHasMore(response.page < response.totalPages && nextPage * PAGE_SIZE < response.total)
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось загрузить еще пользователей.'))
    } finally {
      setIsUsersLoadingMore(false)
    }
  }

  const loadMoreSquads = async () => {
    if (!token || !squadsHasMore || isSquadsLoadingMore) return
    setIsSquadsLoadingMore(true)
    try {
      const nextPage = squadsPage + 1
      const response = await listAdminSquads(token, {
        q: squadsQuery.trim() || undefined,
        page: nextPage,
        perPage: PAGE_SIZE,
      })
      setSquads((prev) => [...prev, ...response.items])
      setSquadsPage(nextPage)
      setSquadsHasMore(response.page * response.perPage < response.total)
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось загрузить еще сквады.'))
    } finally {
      setIsSquadsLoadingMore(false)
    }
  }

  useEffect(() => {
    if (!token || accessState !== 'allowed' || route.type !== 'user') return

    void getAdminUser(token, route.userId)
      .then(setUser)
      .catch((cause) => setError(toDisplayError(cause, 'Ошибка загрузки профиля игрока.')))

    setUserSquad(undefined)
    void getAdminUserSquad(token, route.userId)
      .then(setUserSquad)
      .catch(() => setUserSquad(null))
  }, [accessState, route, token])

  useEffect(() => {
    if (!token || accessState !== 'allowed' || route.type !== 'squad') return

    void Promise.all([getAdminSquad(token, route.squadId), getPublicSquadMembers(route.squadId)])
      .then(([resolvedSquad, members]) => {
        setSquad(resolvedSquad)
        setSquadMembers(members)
      })
      .catch((cause) => setError(toDisplayError(cause, 'Ошибка загрузки профиля сквада.')))
  }, [accessState, route, token])

  if (accessState === 'loading') {
    return (
      <div className="admin-page">
        <LoadingState title="Проверяем доступ к админке" />
      </div>
    )
  }

  if (accessState === 'error') {
    return (
      <div className="admin-page">
        <ErrorState title="Админка недоступна" message={error} />
      </div>
    )
  }

  if (accessState === 'denied') {
    return (
      <div className="admin-page">
        <section className="card admin-state-card">
          <h1 className="card-title">Доступ запрещенн</h1>
        </section>
      </div>
    )
  }

  if (route.type === 'home') {
    return (
      <AdminHome
        token={token!}
        tab={tab}
        users={users}
        squads={squads}
        tokens={tokens}
        usersQuery={usersQuery}
        squadsQuery={squadsQuery}
        usersHasMore={usersHasMore}
        squadsHasMore={squadsHasMore}
        isUsersLoadingMore={isUsersLoadingMore}
        isSquadsLoadingMore={isSquadsLoadingMore}
        onUsersQueryChange={setUsersQuery}
        onSquadsQueryChange={setSquadsQuery}
        onLoadMoreUsers={loadMoreUsers}
        onLoadMoreSquads={loadMoreSquads}
        onTokensChange={setTokens}
      />
    )
  }

  if (route.type === 'user') {
    return <AdminUserProfile token={token!} user={user} userId={route.userId} userSquad={userSquad} onUserChange={setUser} />
  }

  if (route.type === 'tokenAudit') {
    return <AdminTokenAuditView token={token!} tokenId={route.tokenId} />
  }

  if (route.type === 'shopProduct') {
    return <AdminShopProductView token={token!} productId={route.productId} />
  }

  return (
    <AdminSquadProfile token={token!} squad={squad} members={squadMembers} onSquadChange={setSquad} onMembersChange={setSquadMembers} />
  )
}

function AdminHome({
  token,
  tab,
  users,
  squads,
  tokens,
  usersQuery,
  squadsQuery,
  usersHasMore,
  squadsHasMore,
  isUsersLoadingMore,
  isSquadsLoadingMore,
  onUsersQueryChange,
  onSquadsQueryChange,
  onLoadMoreUsers,
  onLoadMoreSquads,
  onTokensChange,
}: {
  token: string
  tab: HomeTab
  users: AdminUserResponse[]
  squads: AdminSquadResponse[]
  tokens: ServiceTokenResponse[]
  usersQuery: string
  squadsQuery: string
  usersHasMore: boolean
  squadsHasMore: boolean
  isUsersLoadingMore: boolean
  isSquadsLoadingMore: boolean
  onUsersQueryChange: (value: string) => void
  onSquadsQueryChange: (value: string) => void
  onLoadMoreUsers: () => Promise<void>
  onLoadMoreSquads: () => Promise<void>
  onTokensChange: (fn: (prev: ServiceTokenResponse[]) => ServiceTokenResponse[]) => void
}) {
  const [systemName, setSystemName] = useState('')
  const [tokenModalSecret, setTokenModalSecret] = useState<string | null>(null)
  const [overviewStats, setOverviewStats] = useState<{
    usersTotal: number
    squadsTotal: number
    serviceTokensTotal: number
    activeServiceTokensTotal: number
  } | null>(null)
  const [overviewError, setOverviewError] = useState('')
  const [defaultSkin, setDefaultSkin] = useState<DefaultSkinResponse | null | undefined>(undefined)
  const [defaultSkinVersion, setDefaultSkinVersion] = useState(() => Date.now())
  const [defaultSkinModel, setDefaultSkinModel] = useState<SkinModel>('default')
  const [isUploadingDefaultSkin, setIsUploadingDefaultSkin] = useState(false)
  const defaultSkinInputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (tab !== 'overview') return
    let cancelled = false

    const run = async () => {
      setOverviewError('')
      setOverviewStats(null)
      setDefaultSkin(undefined)
      try {
        const [usersResponse, squadsResponse, serviceTokensResponse, skinResponse] = await Promise.all([
          listAdminUsers(token, { page: 1, perPage: 1 }),
          listAdminSquads(token, { page: 1, perPage: 1 }),
          listAdminServiceTokens(token),
          getAdminDefaultSkin(token),
        ])
        if (cancelled) return
        setOverviewStats({
          usersTotal: usersResponse.total,
          squadsTotal: squadsResponse.total,
          serviceTokensTotal: serviceTokensResponse.length,
          activeServiceTokensTotal: serviceTokensResponse.filter((item) => item.isActive).length,
        })
        setDefaultSkin(skinResponse)
        setDefaultSkinVersion(Date.now())
      } catch (cause) {
        if (!cancelled) setOverviewError(toDisplayError(cause, 'Ошибка загрузки обзора админки.'))
      }
    }

    void run()
    return () => {
      cancelled = true
    }
  }, [tab, token])

  const createToken = async () => {
    if (!systemName.trim()) {
      toast.error('Укажите systemName.')
      return
    }
    try {
      const created = await createAdminServiceToken(token, { systemName: systemName.trim() })
      onTokensChange((prev) => [created, ...prev.filter((tokenItem) => tokenItem.id !== created.id)])
      setTokenModalSecret(created.plaintextToken ?? null)
      setSystemName('')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось выпустить сервисный токен.'))
    }
  }

  const copySecret = async () => {
    if (!tokenModalSecret) return
    try {
      await navigator.clipboard.writeText(tokenModalSecret)
      toast.success('Токен скопирован.')
    } catch {
      toast.error('Не удалось скопировать токен.')
    }
  }

  const rotateToken = async (tokenId: string) => {
    try {
      const rotated = await rotateAdminServiceToken(token, tokenId, { reason: null })
      onTokensChange((prev) => prev.map((tokenItem) => (tokenItem.id === tokenId ? rotated : tokenItem)))
      setTokenModalSecret(rotated.plaintextToken ?? null)
      toast.success('Токен ротирован.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось ротировать сервисный токен.'))
    }
  }

  const revokeToken = async (tokenId: string) => {
    try {
      const revoked = await revokeAdminServiceToken(token, tokenId, { reason: null })
      onTokensChange((prev) => prev.map((tokenItem) => (tokenItem.id === tokenId ? revoked : tokenItem)))
      toast.success('Токен отозван.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось отозвать сервисный токен.'))
    }
  }

  const uploadDefaultSkin = async (file: File) => {
    if (isUploadingDefaultSkin) return
    setIsUploadingDefaultSkin(true)
    try {
      const uploaded = await uploadAdminDefaultSkin(token, file, defaultSkinModel)
      setDefaultSkin(uploaded)
      setDefaultSkinVersion(Date.now())
      toast.success('Дефолтный скин обновлен.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось обновить дефолтный скин.'))
    } finally {
      setIsUploadingDefaultSkin(false)
      if (defaultSkinInputRef.current) defaultSkinInputRef.current.value = ''
    }
  }

  return (
    <div className="admin-page">
      <section className="admin-top-actions">
        <nav className="admin-tabs">
          <button type="button" className={`admin-tab ${tab === 'overview' ? 'is-active' : ''}`} onClick={() => setHomeTab('overview')}>Обзор</button>
          <button type="button" className={`admin-tab ${tab === 'users' ? 'is-active' : ''}`} onClick={() => setHomeTab('users')}>Пользователи</button>
          <button type="button" className={`admin-tab ${tab === 'squads' ? 'is-active' : ''}`} onClick={() => setHomeTab('squads')}>Сквады</button>
          <button type="button" className={`admin-tab ${tab === 'assets' ? 'is-active' : ''}`} onClick={() => setHomeTab('assets')}>Ассеты</button>
          <button type="button" className={`admin-tab ${tab === 'shop' ? 'is-active' : ''}`} onClick={() => setHomeTab('shop')}>Магазин</button>
          <button type="button" className={`admin-tab ${tab === 'skinShop' ? 'is-active' : ''}`} onClick={() => setHomeTab('skinShop')}>Скины → магазин</button>
          <button type="button" className={`admin-tab ${tab === 'tokens' ? 'is-active' : ''}`} onClick={() => setHomeTab('tokens')}>Сервисные токены</button>
        </nav>
      </section>

      {tab === 'overview' ? (
        <section className="admin-overview">
          {overviewError ? <ErrorState message={overviewError} /> : null}

          {!overviewStats && !overviewError ? <LoadingState title="Загружаем обзор админки" /> : null}

          {overviewStats ? (
            <>
              <div className="admin-stat-grid">
                <article className="admin-stat-card">
                  <span>Пользователи</span>
                  <strong>{overviewStats.usersTotal}</strong>
                </article>
                <article className="admin-stat-card">
                  <span>Сквады</span>
                  <strong>{overviewStats.squadsTotal}</strong>
                </article>
                <article className="admin-stat-card">
                  <span>Сервисные токены</span>
                  <strong>{overviewStats.serviceTokensTotal}</strong>
                  <small>{overviewStats.activeServiceTokensTotal} активных</small>
                </article>
              </div>

              <section className="card admin-card admin-default-skin-card">
                <div className="admin-default-skin-preview">
                  {defaultSkin === undefined ? (
                    <LoadingState title="Загружаем дефолтный скин" />
                  ) : defaultSkin ? (
                    <img
                      className="admin-default-skin-image"
                      src={appendVersion(defaultSkin.imageUrl, defaultSkinVersion)}
                      alt="Дефолтный скин"
                    />
                  ) : (
                    <div className="admin-default-skin-empty">Дефолтный скин не задан.</div>
                  )}
                </div>

                <div className="admin-default-skin-info">
                  <h2 className="card-title">Дефолтный скин</h2>
                  <dl className="admin-kv admin-kv--compact">
                    <div><dt>Формат</dt><dd>{defaultSkin?.contentType ?? '—'}</dd></div>
                    <div><dt>Обновлен</dt><dd>{formatDateTime(defaultSkin?.updatedAt)}</dd></div>
                    <div><dt>Администратор</dt><dd>{defaultSkin?.updatedByUserId ?? '—'}</dd></div>
                  </dl>

                  <div className="admin-default-skin-controls">
                    <div className="admin-default-skin-models" aria-label="Модель скина">
                      <button
                        type="button"
                        className={`btn btn-sm ${defaultSkinModel === 'default' ? 'primary' : ''}`}
                        onClick={() => setDefaultSkinModel('default')}
                      >
                        Обычная
                      </button>
                      <button
                        type="button"
                        className={`btn btn-sm ${defaultSkinModel === 'slim' ? 'primary' : ''}`}
                        onClick={() => setDefaultSkinModel('slim')}
                      >
                        Тонкая
                      </button>
                    </div>
                    <input
                      ref={defaultSkinInputRef}
                      type="file"
                      accept="image/png,image/*"
                      className="admin-hidden-file-input"
                      onChange={(event) => {
                        const file = event.target.files?.[0]
                        if (file) void uploadDefaultSkin(file)
                      }}
                    />
                    <button
                      type="button"
                      className="btn primary"
                      disabled={isUploadingDefaultSkin}
                      onClick={() => defaultSkinInputRef.current?.click()}
                    >
                      {isUploadingDefaultSkin ? 'Загружаем...' : 'Сменить дефолтный скин'}
                    </button>
                  </div>
                </div>
              </section>
            </>
          ) : null}
        </section>
      ) : null}

      {tab === 'users' ? (
        <section className="card admin-card">
          <input className="ui-input" value={usersQuery} onChange={(event) => onUsersQueryChange(event.target.value)} placeholder="Поиск пользователей" />
          <div className="admin-list">
            {users.map((user) => (
              <button key={user.id} type="button" className="admin-row" onClick={() => pushUrl(adminUserPath(user.id))}>
                <span className="admin-row-user">
                  {user.avatarUrl ? <img src={user.avatarUrl} alt={user.username} className="admin-avatar" /> : <span className="ui-avatar ui-avatar-sm">{initials(user.username)}</span>}
                  <span className="admin-row-user-text">
                    <strong>{user.username}</strong>
                    <small>{user.email ?? user.id}</small>
                  </span>
                </span>
                {user.isSuperuser ? (
                  <span className="admin-row-badges">
                    <span className="ui-badge ui-badge-secondary">Superuser</span>
                  </span>
                ) : null}
              </button>
            ))}
          </div>
          {usersHasMore ? (
            <button type="button" className="btn" disabled={isUsersLoadingMore} onClick={() => void onLoadMoreUsers()}>
              {isUsersLoadingMore ? 'Загрузка...' : 'Загрузить еще'}
            </button>
          ) : null}
        </section>
      ) : null}

      {tab === 'squads' ? (
        <section className="card admin-card">
          <h2 className="card-title">Все сквады</h2>
          <input className="ui-input" value={squadsQuery} onChange={(event) => onSquadsQueryChange(event.target.value)} placeholder="Поиск сквадов" />
          <div className="admin-list">
            {squads.map((squadItem) => (
              <button key={squadItem.id} type="button" className="admin-row" onClick={() => pushUrl(adminSquadPath(squadItem.id))}>
                <span className="admin-row-user">
                  {squadItem.imageUrl ? <img src={squadItem.imageUrl} alt={squadItem.name} className="admin-avatar" /> : <span className="ui-avatar ui-avatar-sm">{initials(squadItem.name)}</span>}
                  <span className="admin-row-user-text">
                    <strong>{squadItem.name}</strong>
                    <small>{squadItem.id}</small>
                  </span>
                </span>
                <span className="admin-row-badges">
                  <span className="ui-badge ui-badge-neutral">{squadItem.memberCount}/{squadItem.maxMembers}</span>
                </span>
              </button>
            ))}
          </div>
          {squadsHasMore ? (
            <button type="button" className="btn" disabled={isSquadsLoadingMore} onClick={() => void onLoadMoreSquads()}>
              {isSquadsLoadingMore ? 'Загрузка...' : 'Загрузить еще'}
            </button>
          ) : null}
        </section>
      ) : null}

      {tab === 'tokens' ? (
        <section className="card admin-card">
          <div className="admin-token-create">
            <input className="ui-input" value={systemName} onChange={(event) => setSystemName(event.target.value.toLowerCase())} placeholder="system_name" />
            <button className="btn primary" type="button" onClick={() => void createToken()}>Выдать</button>
          </div>

          <div className="admin-token-list">
            {tokens.map((tokenItem) => (
              <article key={tokenItem.id} className="admin-token-card">
                <div className="admin-token-head">
                  <strong>{tokenItem.systemName}</strong>
                  <span className={`ui-badge ${tokenItem.isActive ? 'ui-badge-success' : 'ui-badge-warning'}`}>
                    {tokenItem.isActive ? 'Активен' : 'Отозван'}
                  </span>
                </div>
                <small>{tokenItem.id}</small>
                <div className="admin-token-actions">
                  <button type="button" className="btn btn-sm" disabled={!tokenItem.isActive} onClick={() => void rotateToken(tokenItem.id)}>
                    Ротация
                  </button>
                  <button type="button" className="btn btn-sm danger" onClick={() => void revokeToken(tokenItem.id)}>
                    Отзыв
                  </button>
                  <button type="button" className="btn btn-sm" onClick={() => pushUrl(adminTokenAuditPath(tokenItem.id))}>
                    Аудит
                  </button>
                </div>
              </article>
            ))}
          </div>
        </section>
      ) : null}

      {tab === 'assets' ? <AdminAssetsPanel token={token} /> : null}

      {tab === 'shop' ? <AdminShopPanel token={token} /> : null}
      {tab === 'skinShop' ? <AdminGunskinShopWizard token={token} /> : null}

      {tokenModalSecret ? (
        <div className="admin-token-modal-backdrop" role="dialog" aria-modal="true" aria-label="Новый сервисный токен">
          <div className="card admin-token-modal">
            <h3 className="card-title">Ваш сервисный токен</h3>
            <p className="card-text">Вы увидите его только в этом окне. Сохраните значение сейчас.</p>
            <div className="admin-token-modal-row">
              <input className="ui-input admin-token-modal-input" value={tokenModalSecret} disabled />
              <button type="button" className="btn" onClick={() => void copySecret()}>Скопировать</button>
            </div>
            <div className="admin-token-modal-actions">
              <button type="button" className="btn primary" onClick={() => setTokenModalSecret(null)}>Закрыть</button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  )
}

function AdminTokenAuditView({ token, tokenId }: { token: string; tokenId: string }) {
  const [items, setItems] = useState<ServiceTokenAuditResponse[] | null>(null)
  const [error, setError] = useState('')

  useEffect(() => {
    let cancelled = false
    const run = async () => {
      try {
        const response = await getAdminServiceTokenAudit(token, tokenId)
        if (!cancelled) setItems(response)
      } catch (cause) {
        if (!cancelled) setError(toDisplayError(cause, 'Не удалось загрузить аудит токена.'))
      }
    }
    void run()
    return () => {
      cancelled = true
    }
  }, [token, tokenId])

  return (
    <div className="admin-page">
      <section className="admin-top-actions">
        <button type="button" className="btn btn-sm" onClick={() => pushUrl(`${paths.admin}?tab=tokens`)}>
          ← К токенам
        </button>
      </section>

      <section className="card admin-card">
        <h2 className="card-title">Аудит токена</h2>
        <p className="card-text">Token ID: <code>{tokenId}</code></p>
        {!items && !error ? <LoadingState title="Загружаем аудит" /> : null}
        {error ? <ErrorState message={error} /> : null}
        {items ? (
          items.length > 0 ? (
            <div className="admin-audit-table-wrap">
              <table className="admin-audit-table">
                <thead>
                  <tr>
                    <th>ID</th>
                    <th>Action</th>
                    <th>Actor</th>
                    <th>Reason</th>
                    <th>Metadata</th>
                    <th>Created At</th>
                  </tr>
                </thead>
                <tbody>
                  {items.map((item) => (
                    <tr key={item.id}>
                      <td>{item.id}</td>
                      <td>{item.action}</td>
                      <td>{item.actorUserId}</td>
                      <td>{item.reason ?? '—'}</td>
                      <td className="admin-audit-metadata" title={formatMetadata(item.metadata)}>{formatMetadata(item.metadata)}</td>
                      <td>{formatDateTime(item.createdAt)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : <p className="admin-inline-muted">Записей аудита нет.</p>
        ) : null}
      </section>
    </div>
  )
}

function AdminUserView({
  token,
  user,
  userId,
  userSquad,
  onUserChange,
}: {
  token: string
  user: AdminUserResponse | null
  userId: string
  userSquad: AdminSquadResponse | null | undefined
  onUserChange: (value: AdminUserResponse | null) => void
}) {
  const [skinVersion, setSkinVersion] = useState(() => Date.now())
  const [isDeletingSkin, setIsDeletingSkin] = useState(false)
  const [isUpdatingSuperuser, setIsUpdatingSuperuser] = useState(false)

  const deleteSkin = async () => {
    if (!user || isDeletingSkin) return
    setIsDeletingSkin(true)
    try {
      await deleteAdminUserSkin(token, user.id)
      setSkinVersion(Date.now())
      toast.success('Скин пользователя удален.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось удалить скин пользователя.'))
    } finally {
      setIsDeletingSkin(false)
    }
  }

  const grantSuperuser = async () => {
    if (!user || isUpdatingSuperuser) return
    setIsUpdatingSuperuser(true)
    try {
      const updated = await grantAdminUserSuperuser(token, user.id)
      onUserChange(updated)
      toast.success('Админ-права выданы.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось выдать админ-права.'))
    } finally {
      setIsUpdatingSuperuser(false)
    }
  }

  const revokeSuperuser = async () => {
    if (!user || isUpdatingSuperuser) return
    setIsUpdatingSuperuser(true)
    try {
      const updated = await revokeAdminUserSuperuser(token, user.id)
      onUserChange(updated)
      toast.success('Админ-права сняты.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось снять админ-права.'))
    } finally {
      setIsUpdatingSuperuser(false)
    }
  }

  return (
    <div className="admin-page">
      <section className="admin-top-actions">
        <button type="button" className="btn btn-sm" onClick={() => pushUrl(`${paths.admin}?tab=users`)}>
          ← К пользователям
        </button>
      </section>

      <section className="card admin-card">
        {!user ? <LoadingState title="Загружаем профиль игрока" /> : (
          <div className="admin-user-layout">
            <aside className="admin-user-side">
              <div className="admin-row-user">
                {user.avatarUrl ? <img src={user.avatarUrl} alt={user.username} className="admin-avatar admin-avatar-lg" /> : <span className="ui-avatar">{initials(user.username)}</span>}
                <div className="admin-row-user-text">
                  <h2 className="card-title">{user.username}</h2>
                  <small>{userId}</small>
                </div>
              </div>
              <img className="admin-user-skin" src={buildSkinUrl(user.id, skinVersion)} alt={`Скин ${user.username}`} />
              <button className="btn danger" type="button" disabled={isDeletingSkin} onClick={() => void deleteSkin()}>
                {isDeletingSkin ? 'Удаляем...' : 'Удалить скин'}
              </button>
              <div className="admin-user-role-actions">
                <button className="btn btn-sm" type="button" disabled={isUpdatingSuperuser || user.isSuperuser} onClick={() => void grantSuperuser()}>
                  Выдать Superuser
                </button>
                <button className="btn btn-sm danger" type="button" disabled={isUpdatingSuperuser || !user.isSuperuser} onClick={() => void revokeSuperuser()}>
                  Снять Superuser
                </button>
              </div>
            </aside>

            <dl className="admin-kv">
              <div><dt>Email</dt><dd>{user.email ?? 'Не указан'}</dd></div>
              <div><dt>Discord ID</dt><dd>{user.discordId}</dd></div>
              <div><dt>Последний вход</dt><dd>{formatDateTime(user.lastLoginAt)}</dd></div>
              <div><dt>Создан</dt><dd>{formatDateTime(user.createdAt)}</dd></div>
              <div>
                <dt>Сквад</dt>
                <dd>{userSquad === undefined ? 'Поиск...' : userSquad ? <button type="button" className="btn btn-sm" onClick={() => pushUrl(adminSquadPath(userSquad.id))}>{userSquad.name}</button> : 'Не состоит в скваде'}</dd>
              </div>
            </dl>
          </div>
        )}
      </section>
    </div>
  )
}

void AdminUserView

function AdminSquadView({
  token,
  squad,
  members,
  onSquadChange,
}: {
  token: string
  squad: AdminSquadResponse | null
  members: SquadMemberResponse[]
  onSquadChange: (value: AdminSquadResponse | null) => void
}) {
  const activeMembers = members.filter((member) => !member.isPendingInvite)
  const outgoingInvites = members.filter((member) => member.isPendingInvite)
  const [restrictionReason, setRestrictionReason] = useState('')
  const [isUpdatingRestriction, setIsUpdatingRestriction] = useState(false)
  const [squadName, setSquadName] = useState('')
  const [isEditingName, setIsEditingName] = useState(false)
  const [isUpdatingName, setIsUpdatingName] = useState(false)
  const [isDeletingSquad, setIsDeletingSquad] = useState(false)

  useEffect(() => {
    setSquadName(squad?.name ?? '')
  }, [squad?.id, squad?.name])

  const deleteSquadAvatar = async () => {
    if (!squad) return
    try {
      const updated = await deleteAdminSquadImage(token, squad.id)
      onSquadChange(updated)
      toast.success('Аватарка сквада удалена.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось удалить аватарку сквада.'))
    }
  }

  const restrictSquad = async () => {
    if (!squad || isUpdatingRestriction) return
    setIsUpdatingRestriction(true)
    try {
      const updated = await restrictAdminSquad(token, squad.id, {
        reason: restrictionReason.trim() || null,
      })
      onSquadChange(updated)
      toast.success('Ограничение на сквад выдано.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось выдать ограничение на сквад.'))
    } finally {
      setIsUpdatingRestriction(false)
    }
  }

  const unrestrictSquad = async () => {
    if (!squad || isUpdatingRestriction) return
    setIsUpdatingRestriction(true)
    try {
      const updated = await unrestrictAdminSquad(token, squad.id, {
        reason: restrictionReason.trim() || null,
      })
      onSquadChange(updated)
      toast.success('Ограничение со сквада снято.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось снять ограничение со сквада.'))
    } finally {
      setIsUpdatingRestriction(false)
    }
  }

  const updateSquadName = async () => {
    if (!squad || isUpdatingName) return
    const normalizedName = squadName.trim()
    if (!normalizedName) {
      toast.error('Укажите название сквада.')
      return
    }
    if (normalizedName === squad.name) return

    setIsUpdatingName(true)
    try {
      const updated = await patchAdminSquad(token, squad.id, { name: normalizedName })
      onSquadChange(updated)
      setSquadName(updated.name)
      setIsEditingName(false)
      toast.success('Название сквада обновлено.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось обновить название сквада.'))
    } finally {
      setIsUpdatingName(false)
    }
  }

  const cancelSquadNameEdit = () => {
    setSquadName(squad?.name ?? '')
    setIsEditingName(false)
  }

  const removeSquad = async () => {
    if (!squad || isDeletingSquad) return
    const approved = window.confirm(`Удалить сквад "${squad.name}"? Это действие нельзя отменить.`)
    if (!approved) return

    setIsDeletingSquad(true)
    try {
      await deleteAdminSquad(token, squad.id)
      toast.success('Сквад удален.')
      pushUrl(`${paths.admin}?tab=squads`)
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось удалить сквад.'))
    } finally {
      setIsDeletingSquad(false)
    }
  }

  return (
    <div className="admin-page">
      <section className="admin-top-actions">
        <button type="button" className="btn btn-sm" onClick={() => pushUrl(`${paths.admin}?tab=squads`)}>
          ← К сквадам
        </button>
      </section>

      <section className="card admin-card">
        {!squad ? <LoadingState title="Загружаем профиль сквада" /> : (
          <>
            <div className="admin-squad-head">
              {squad.imageUrl ? <img src={squad.imageUrl} alt={squad.name} className="admin-avatar admin-avatar-lg" /> : <span className="ui-avatar">{initials(squad.name)}</span>}
              <div className="admin-row-user-text">
                {!isEditingName ? (
                  <div className="admin-squad-name-row">
                    <h2 className="card-title">{squad.name}</h2>
                    <button
                      type="button"
                      className="btn btn-sm"
                      disabled={isUpdatingName || isDeletingSquad}
                      onClick={() => setIsEditingName(true)}
                    >
                      Редактировать
                    </button>
                  </div>
                ) : (
                  <div className="admin-squad-name-edit">
                    <input
                      className="ui-input"
                      value={squadName}
                      onChange={(event) => setSquadName(event.target.value)}
                      placeholder="Название сквада"
                      autoFocus
                    />
                    <div className="admin-squad-name-edit-actions">
                      <button
                        type="button"
                        className="btn btn-sm"
                        disabled={isUpdatingName || !squadName.trim() || squadName.trim() === squad.name}
                        onClick={() => void updateSquadName()}
                      >
                        {isUpdatingName ? 'Сохраняем...' : 'Сохранить'}
                      </button>
                      <button
                        type="button"
                        className="btn btn-sm"
                        disabled={isUpdatingName}
                        onClick={cancelSquadNameEdit}
                      >
                        Отмена
                      </button>
                    </div>
                  </div>
                )}
                <small>{squad.id}</small>
              </div>
              <button className="btn danger" type="button" disabled={!squad.imageUrl} onClick={() => void deleteSquadAvatar()}>
                Удалить аватарку сквада
              </button>
            </div>

            <dl className="admin-kv">
              <div><dt>Создан</dt><dd>{formatDateTime(squad.createdAt)}</dd></div>
              <div><dt>Обновлён</dt><dd>{formatDateTime(squad.updatedAt)}</dd></div>
              <div><dt>Лидер</dt><dd>{squad.leaderUserId}</dd></div>
            </dl>

            <section className="admin-card-subsection">
              <h3 className="card-title">Ограничение сквада</h3>
              <div className="admin-restriction-status">
                <span className={`ui-badge ${squad.isRestricted ? 'ui-badge-warning' : 'ui-badge-success'}`}>
                  {squad.isRestricted ? 'Сквад ограничен' : 'Ограничений нет'}
                </span>
                {squad.restrictionReason ? <small>{squad.restrictionReason}</small> : null}
              </div>
              <div className="admin-restriction-toolbar">
                <input
                  className="ui-input"
                  value={restrictionReason}
                  onChange={(event) => setRestrictionReason(event.target.value)}
                  placeholder="Причина ограничения"
                />
                <button
                  type="button"
                  className="btn btn-sm danger"
                  disabled={isUpdatingRestriction || squad.isRestricted}
                  onClick={() => void restrictSquad()}
                >
                  Выдать ограничение
                </button>
                <button
                  type="button"
                  className="btn btn-sm"
                  disabled={isUpdatingRestriction || !squad.isRestricted}
                  onClick={() => void unrestrictSquad()}
                >
                  Снять ограничение
                </button>
              </div>
            </section>

            <section className="admin-card-subsection">
              <h3 className="card-title">Управление сквадом</h3>
              <div className="admin-squad-manage-row">
                <input
                  className="ui-input"
                  value={squadName}
                  onChange={(event) => setSquadName(event.target.value)}
                  placeholder="Название сквада"
                />
                <button
                  type="button"
                  className="btn btn-sm"
                  disabled={isUpdatingName || !squadName.trim() || squadName.trim() === squad.name}
                  onClick={() => void updateSquadName()}
                >
                  {isUpdatingName ? 'Сохраняем...' : 'Сохранить название'}
                </button>
                <button
                  type="button"
                  className="btn btn-sm danger"
                  disabled={isDeletingSquad}
                  onClick={() => void removeSquad()}
                >
                  {isDeletingSquad ? 'Удаляем...' : 'Удалить сквад'}
                </button>
              </div>
            </section>

            <div className="admin-squad-columns">
              <section>
                <h3>Участники</h3>
                <ul className="admin-member-list">
                  {activeMembers.map((member) => (
                    <li key={member.id}>
                      <div className="admin-member-card-main" onClick={() => pushUrl(adminUserPath(member.id))}>
                        {member.avatarUrl ? <img src={member.avatarUrl} alt={member.username} className="admin-avatar" /> : <span className="ui-avatar ui-avatar-sm">{initials(member.username)}</span>}
                        <span className="admin-member-name">{member.username}</span>
                      </div>
                      {member.id === squad.leaderUserId && <span className="ui-badge ui-badge-secondary">Лидер</span>}
                    </li>
                  ))}
                </ul>
              </section>

              <section>
                <h3>Исходящие запросы</h3>
                <ul className="admin-member-list">
                  {outgoingInvites.map((member) => (
                    <li key={member.inviteId ?? member.id}>
                      <div className="admin-member-card-main" onClick={() => pushUrl(adminUserPath(member.id))}>
                        {member.avatarUrl ? <img src={member.avatarUrl} alt={member.username} className="admin-avatar" /> : <span className="ui-avatar ui-avatar-sm">{initials(member.username)}</span>}
                        <span className="admin-member-name">{member.username}</span>
                      </div>
                      <span className="ui-badge ui-badge-warning">Invite</span>
                    </li>
                  ))}
                  {outgoingInvites.length === 0 ? <li className="admin-inline-muted">Нет исходящих инвайтов.</li> : null}
                </ul>
              </section>
            </div>
          </>
        )}
      </section>
    </div>
  )
}

void AdminSquadView
