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
import { adminLittlemicePath, adminSquadPath, adminTokenAuditPath, adminUserPath, paths } from '../../routes/paths'
import { pushUrl, usePathname } from '../../shared/navigation/history'
import { getAuthToken } from '../../shared/session/auth-session'
import { useQuery } from '../../util/query'
import AdminAssetsPanel from './AdminAssetsPanel'
import AdminAssetView from './AdminAssetView'
import AdminGunskinShopWizard from './AdminGunskinShopWizard'
import AdminLootboxesPanel from './AdminLootboxesPanel'
import AdminLootboxView from './AdminLootboxView'
import AdminLittlemiceTimelineView from './AdminLittlemiceTimelineView'
import AdminShopPanel from './AdminShopPanel'
import AdminShopProductView from './AdminShopProductView'
import AdminSquadProfile from './AdminSquadProfile'
import AdminUserLittlemiceChecksView from './AdminUserLittlemiceChecksView'
import AdminUserLootboxHistoryView from './AdminUserLootboxHistoryView'
import AdminUserProfile from './AdminUserProfile'
import AdminLink from './AdminLink'
import ErrorState from '../ErrorState'
import LoadingState from '../LoadingState'
import './AdminPage.css'

type AdminRoute =
  | { type: 'home' }
  | { type: 'user'; userId: string }
  | { type: 'squad'; squadId: string }
  | { type: 'tokenAudit'; tokenId: string }
  | { type: 'asset'; assetId: string }
  | { type: 'shopProduct'; productId: string }
  | { type: 'lootbox'; lootboxId: string }
  | { type: 'littlemice' }
  | { type: 'userLootboxHistory'; userId: string }
  | { type: 'userLittlemice'; userId: string; checkId?: string }
type AccessState = 'loading' | 'allowed' | 'denied' | 'error'
type HomeTab = 'overview' | 'users' | 'squads' | 'tokens' | 'assets' | 'shop' | 'skinShop' | 'lootboxes'
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

  const userLootboxHistoryMatch = pathname.match(/^\/admin\/users\/([^/]+)\/lootboxes\/open-history$/)
  if (userLootboxHistoryMatch) return { type: 'userLootboxHistory', userId: decodeURIComponent(userLootboxHistoryMatch[1]) }

  const userLittlemiceCheckMatch = pathname.match(/^\/admin\/users\/([^/]+)\/littlemice\/([^/]+)$/)
  if (userLittlemiceCheckMatch) {
    return {
      type: 'userLittlemice',
      userId: decodeURIComponent(userLittlemiceCheckMatch[1]),
      checkId: decodeURIComponent(userLittlemiceCheckMatch[2]),
    }
  }

  const userLittlemiceMatch = pathname.match(/^\/admin\/users\/([^/]+)\/littlemice$/)
  if (userLittlemiceMatch) return { type: 'userLittlemice', userId: decodeURIComponent(userLittlemiceMatch[1]) }

  const squadMatch = pathname.match(/^\/admin\/squads\/([^/]+)$/)
  if (squadMatch) return { type: 'squad', squadId: decodeURIComponent(squadMatch[1]) }

  const auditMatch = pathname.match(/^\/admin\/tokens\/([^/]+)\/audit$/)
  if (auditMatch) return { type: 'tokenAudit', tokenId: decodeURIComponent(auditMatch[1]) }

  const assetMatch = pathname.match(/^\/admin\/assets\/([^/]+)$/)
  if (assetMatch) return { type: 'asset', assetId: decodeURIComponent(assetMatch[1]) }

  const shopProductMatch = pathname.match(/^\/admin\/shop\/products\/([^/]+)$/)
  if (shopProductMatch) return { type: 'shopProduct', productId: decodeURIComponent(shopProductMatch[1]) }

  const lootboxMatch = pathname.match(/^\/admin\/lootboxes\/([^/]+)$/)
  if (lootboxMatch) return { type: 'lootbox', lootboxId: decodeURIComponent(lootboxMatch[1]) }

  if (pathname === adminLittlemicePath()) return { type: 'littlemice' }

  return { type: 'home' }
}

function tabFromQuery(tab: string | null): HomeTab {
  if (tab === 'overview') return 'overview'
  if (tab === 'squads') return 'squads'
  if (tab === 'tokens') return 'tokens'
  if (tab === 'assets') return 'assets'
  if (tab === 'shop') return 'shop'
  if (tab === 'skinShop') return 'skinShop'
  if (tab === 'lootboxes') return 'lootboxes'
  if (tab === 'users') return 'users'
  return 'overview'
}

function adminHomeTabPath(tab: HomeTab) {
  return `${paths.admin}?tab=${tab}`
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
  const [usersTotal, setUsersTotal] = useState(0)
  const [squadsTotal, setSquadsTotal] = useState(0)
  const [usersTotalPages, setUsersTotalPages] = useState(1)
  const [squadsTotalPages, setSquadsTotalPages] = useState(1)

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
      void listAdminUsers(token, { q: usersQuery.trim() || undefined, page: usersPage, perPage: PAGE_SIZE })
        .then((response) => {
          setUsers(response.items)
          setUsersTotal(response.total)
          setUsersTotalPages(Math.max(1, response.totalPages))
        })
        .catch((cause) => setError(toDisplayError(cause, 'Ошибка загрузки пользователей.')))
    }

    if (tab === 'squads') {
      void listAdminSquads(token, { q: squadsQuery.trim() || undefined, page: squadsPage, perPage: PAGE_SIZE })
        .then((response) => {
          setSquads(response.items)
          setSquadsTotal(response.total)
          setSquadsTotalPages(Math.max(1, Math.ceil(response.total / response.perPage)))
        })
        .catch((cause) => setError(toDisplayError(cause, 'Ошибка загрузки сквадов.')))
    }

    if (tab === 'tokens') {
      void listAdminServiceTokens(token)
        .then(setTokens)
        .catch((cause) => setError(toDisplayError(cause, 'Ошибка загрузки сервисных токенов.')))
    }
  }, [accessState, route.type, tab, token, usersQuery, usersPage, squadsQuery, squadsPage])

  const changeUsersQuery = (value: string) => {
    setUsersQuery(value)
    setUsersPage(1)
  }

  const changeSquadsQuery = (value: string) => {
    setSquadsQuery(value)
    setSquadsPage(1)
  }

  useEffect(() => {
    if (!token || accessState !== 'allowed' || route.type !== 'user') return

    void getAdminUser(token, route.userId)
      .then(setUser)
      .catch((cause) => setError(toDisplayError(cause, 'Ошибка загрузки профиля игрока.')))

    queueMicrotask(() => setUserSquad(undefined))
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
          <h1 className="card-title">Доступ запрещен</h1>
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
        usersPage={usersPage}
        squadsPage={squadsPage}
        usersTotal={usersTotal}
        squadsTotal={squadsTotal}
        usersTotalPages={usersTotalPages}
        squadsTotalPages={squadsTotalPages}
        onUsersQueryChange={changeUsersQuery}
        onSquadsQueryChange={changeSquadsQuery}
        onUsersPageChange={setUsersPage}
        onSquadsPageChange={setSquadsPage}
        onTokensChange={setTokens}
      />
    )
  }

  if (route.type === 'user') {
    return <AdminUserProfile token={token!} user={user} userId={route.userId} userSquad={userSquad} onUserChange={setUser} />
  }

  if (route.type === 'userLootboxHistory') {
    return <AdminUserLootboxHistoryView token={token!} userId={route.userId} />
  }

  if (route.type === 'userLittlemice') {
    return <AdminUserLittlemiceChecksView token={token!} userId={route.userId} checkId={route.checkId} />
  }

  if (route.type === 'tokenAudit') {
    return <AdminTokenAuditView token={token!} tokenId={route.tokenId} />
  }

  if (route.type === 'asset') {
    return <AdminAssetView token={token!} assetId={route.assetId} />
  }

  if (route.type === 'shopProduct') {
    return <AdminShopProductView token={token!} productId={route.productId} />
  }

  if (route.type === 'lootbox') {
    return <AdminLootboxView token={token!} lootboxId={route.lootboxId} />
  }

  if (route.type === 'littlemice') {
    return <AdminLittlemiceTimelineView token={token!} />
  }

  return (
    <AdminSquadProfile token={token!} squad={squad} members={squadMembers} onSquadChange={setSquad} onMembersChange={setSquadMembers} />
  )
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

function AdminHome({
  token,
  tab,
  users,
  squads,
  tokens,
  usersQuery,
  squadsQuery,
  usersPage,
  squadsPage,
  usersTotal,
  squadsTotal,
  usersTotalPages,
  squadsTotalPages,
  onUsersQueryChange,
  onSquadsQueryChange,
  onUsersPageChange,
  onSquadsPageChange,
  onTokensChange,
}: {
  token: string
  tab: HomeTab
  users: AdminUserResponse[]
  squads: AdminSquadResponse[]
  tokens: ServiceTokenResponse[]
  usersQuery: string
  squadsQuery: string
  usersPage: number
  squadsPage: number
  usersTotal: number
  squadsTotal: number
  usersTotalPages: number
  squadsTotalPages: number
  onUsersQueryChange: (value: string) => void
  onSquadsQueryChange: (value: string) => void
  onUsersPageChange: (page: number) => void
  onSquadsPageChange: (page: number) => void
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
          <AdminLink href={adminHomeTabPath('overview')} replace className={`admin-tab ${tab === 'overview' ? 'is-active' : ''}`}>Обзор</AdminLink>
          <AdminLink href={adminHomeTabPath('users')} replace className={`admin-tab ${tab === 'users' ? 'is-active' : ''}`}>Пользователи</AdminLink>
          <AdminLink href={adminHomeTabPath('squads')} replace className={`admin-tab ${tab === 'squads' ? 'is-active' : ''}`}>Сквады</AdminLink>
          <AdminLink href={adminHomeTabPath('assets')} replace className={`admin-tab ${tab === 'assets' ? 'is-active' : ''}`}>Ассеты</AdminLink>
          <AdminLink href={adminHomeTabPath('shop')} replace className={`admin-tab ${tab === 'shop' ? 'is-active' : ''}`}>Магазин</AdminLink>
          <AdminLink href={adminHomeTabPath('skinShop')} replace className={`admin-tab ${tab === 'skinShop' ? 'is-active' : ''}`}>Скины → магазин</AdminLink>
          <AdminLink href={adminHomeTabPath('lootboxes')} replace className={`admin-tab ${tab === 'lootboxes' ? 'is-active' : ''}`}>Лутбоксы</AdminLink>
          <AdminLink href={adminLittlemicePath()} className="admin-tab">Littlemice</AdminLink>
          <AdminLink href={adminHomeTabPath('tokens')} replace className={`admin-tab ${tab === 'tokens' ? 'is-active' : ''}`}>Сервисные токены</AdminLink>
        </nav>
      </section>

      {tab === 'overview' ? (
        <section className="admin-overview">
          {overviewError ? <ErrorState message={overviewError} /> : null}

          {!overviewStats && !overviewError ? <LoadingState title="Загружаем обзор админки" /> : null}

          {overviewStats ? (
            <>
              <div className="admin-stat-grid">
                <AdminLink className="admin-stat-card" href={adminHomeTabPath('users')}>
                  <span>Пользователи</span>
                  <strong>{overviewStats.usersTotal}</strong>
                </AdminLink>
                <AdminLink className="admin-stat-card" href={adminHomeTabPath('squads')}>
                  <span>Сквады</span>
                  <strong>{overviewStats.squadsTotal}</strong>
                </AdminLink>
                <AdminLink className="admin-stat-card" href={adminHomeTabPath('tokens')}>
                  <span>Сервисные токены</span>
                  <strong>{overviewStats.serviceTokensTotal}</strong>
                  <small>{overviewStats.activeServiceTokensTotal} активных</small>
                </AdminLink>
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
          <div className="admin-section-head">
            <div>
              <h2 className="card-title">Пользователи</h2>
              <p className="card-text">Поиск, профиль игрока, ограничения, инвентарь и проверки.</p>
            </div>
            <span className="ui-badge ui-badge-neutral">Страница {usersPage} из {Math.max(1, usersTotalPages)}</span>
          </div>
          <div className="admin-metric-strip admin-metric-strip--compact">
            <div className="admin-metric">
              <span>Найдено</span>
              <strong>{usersTotal}</strong>
            </div>
            <div className="admin-metric">
              <span>На странице</span>
              <strong>{users.length}</strong>
            </div>
            <div className="admin-metric">
              <span>Superuser</span>
              <strong>{users.filter((item) => item.isSuperuser).length}</strong>
            </div>
          </div>
          <div className="admin-filter-bar">
            <label className="admin-shop-field">
              <span>Поиск</span>
              <input className="ui-input" value={usersQuery} onChange={(event) => onUsersQueryChange(event.target.value)} placeholder="Ник, email или UUID" />
            </label>
          </div>
          <div className="admin-list">
            {users.map((user) => (
              <AdminLink key={user.id} href={adminUserPath(user.id)} className="admin-row">
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
              </AdminLink>
            ))}
            {users.length === 0 ? <p className="admin-inline-muted">Пользователи не найдены.</p> : null}
          </div>
          <AdminPagination page={usersPage} totalPages={Math.max(1, usersTotalPages)} onPageChange={onUsersPageChange} label="Нумерация страниц пользователей" />
        </section>
      ) : null}

      {tab === 'squads' ? (
        <section className="card admin-card">
          <div className="admin-section-head">
            <div>
              <h2 className="card-title">Сквады</h2>
              <p className="card-text">Составы, лидеры, ограничения и управление аватарками.</p>
            </div>
            <span className="ui-badge ui-badge-neutral">Страница {squadsPage} из {Math.max(1, squadsTotalPages)}</span>
          </div>
          <div className="admin-metric-strip admin-metric-strip--compact">
            <div className="admin-metric">
              <span>Найдено</span>
              <strong>{squadsTotal}</strong>
            </div>
            <div className="admin-metric">
              <span>На странице</span>
              <strong>{squads.length}</strong>
            </div>
            <div className="admin-metric admin-metric--warning">
              <span>Ограничены</span>
              <strong>{squads.filter((item) => item.isRestricted).length}</strong>
            </div>
          </div>
          <div className="admin-filter-bar">
            <label className="admin-shop-field">
              <span>Поиск</span>
              <input className="ui-input" value={squadsQuery} onChange={(event) => onSquadsQueryChange(event.target.value)} placeholder="Название или ID сквада" />
            </label>
          </div>
          <div className="admin-list">
            {squads.map((squadItem) => (
              <AdminLink key={squadItem.id} href={adminSquadPath(squadItem.id)} className="admin-row">
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
              </AdminLink>
            ))}
            {squads.length === 0 ? <p className="admin-inline-muted">Сквады не найдены.</p> : null}
          </div>
          <AdminPagination page={squadsPage} totalPages={Math.max(1, squadsTotalPages)} onPageChange={onSquadsPageChange} label="Нумерация страниц сквадов" />
        </section>
      ) : null}

      {tab === 'tokens' ? (
        <section className="card admin-card">
          <div className="admin-section-head">
            <div>
              <h2 className="card-title">Сервисные токены</h2>
              <p className="card-text">Интеграционные токены, ротация, отзыв и аудит действий.</p>
            </div>
          </div>
          <div className="admin-metric-strip admin-metric-strip--compact">
            <div className="admin-metric">
              <span>Всего</span>
              <strong>{tokens.length}</strong>
            </div>
            <div className="admin-metric admin-metric--success">
              <span>Активны</span>
              <strong>{tokens.filter((item) => item.isActive).length}</strong>
            </div>
            <div className="admin-metric admin-metric--warning">
              <span>Отозваны</span>
              <strong>{tokens.filter((item) => !item.isActive).length}</strong>
            </div>
          </div>
          <div className="admin-token-create admin-filter-bar">
            <label className="admin-shop-field">
              <span>systemName</span>
              <input className="ui-input" value={systemName} onChange={(event) => setSystemName(event.target.value.toLowerCase())} placeholder="discord-worker" />
            </label>
            <button className="btn primary" type="button" onClick={() => void createToken()}>Выдать токен</button>
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
                  <AdminLink className="btn btn-sm" href={adminTokenAuditPath(tokenItem.id)}>
                    Аудит
                  </AdminLink>
                </div>
              </article>
            ))}
          </div>
        </section>
      ) : null}

      {tab === 'assets' ? <AdminAssetsPanel token={token} /> : null}

      {tab === 'shop' ? <AdminShopPanel token={token} /> : null}
      {tab === 'skinShop' ? <AdminGunskinShopWizard token={token} /> : null}
      {tab === 'lootboxes' ? <AdminLootboxesPanel token={token} /> : null}

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
        <AdminLink className="btn btn-sm" href={`${paths.admin}?tab=tokens`}>
          ← К токенам
        </AdminLink>
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
        <AdminLink className="btn btn-sm" href={`${paths.admin}?tab=users`}>
          ← К пользователям
        </AdminLink>
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
                <dd>{userSquad === undefined ? 'Поиск...' : userSquad ? <AdminLink className="btn btn-sm" href={adminSquadPath(userSquad.id)}>{userSquad.name}</AdminLink> : 'Не состоит в скваде'}</dd>
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
        <AdminLink className="btn btn-sm" href={`${paths.admin}?tab=squads`}>
          ← К сквадам
        </AdminLink>
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
                      <AdminLink className="admin-member-card-main" href={adminUserPath(member.id)}>
                        {member.avatarUrl ? <img src={member.avatarUrl} alt={member.username} className="admin-avatar" /> : <span className="ui-avatar ui-avatar-sm">{initials(member.username)}</span>}
                        <span className="admin-member-name">{member.username}</span>
                      </AdminLink>
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
                      <AdminLink className="admin-member-card-main" href={adminUserPath(member.id)}>
                        {member.avatarUrl ? <img src={member.avatarUrl} alt={member.username} className="admin-avatar" /> : <span className="ui-avatar ui-avatar-sm">{initials(member.username)}</span>}
                        <span className="admin-member-name">{member.username}</span>
                      </AdminLink>
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
