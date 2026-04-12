import { useEffect, useMemo, useState } from 'react'
import toast from 'react-hot-toast'
import {
  activateAdminUser,
  deactivateAdminUser,
  deleteAdminUserSkin,
  getAdminUserRestrictions,
  grantAdminUserSuperuser,
  grantAdminUserRestriction,
  revokeAdminUserSuperuser,
  revokeAdminUserRestriction,
  type AdminSquadResponse,
  type AdminUserResponse,
  type AdminUserRestrictionResponse,
} from '../../api/admin'
import { toDisplayError } from '../../api/http'
import { getRestrictionMeta, type RestrictionMetaResponse } from '../../api/meta'
import { buildSkinUrl } from '../../api/skins'
import { adminSquadPath, paths } from '../../routes/paths'
import { pushUrl } from '../../shared/navigation/history'
import LoadingState from '../LoadingState'
import SkinViewer3D from '../SkinViewer3D'

function formatDateTime(value?: string | null) {
  if (!value) return 'Нет данных'
  const date = new Date(value)
  return Number.isNaN(date.getTime())
    ? value
    : new Intl.DateTimeFormat('ru-RU', {
      day: '2-digit',
      month: 'short',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    }).format(date)
}

function initials(value: string) {
  return value.slice(0, 2).toUpperCase()
}

export default function AdminUserProfile({
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
  const [isUpdatingActivity, setIsUpdatingActivity] = useState(false)
  const [isUpdatingRestrictions, setIsUpdatingRestrictions] = useState(false)
  const [restrictionReason, setRestrictionReason] = useState('')
  const [userRestrictions, setUserRestrictions] = useState<AdminUserRestrictionResponse[]>([])
  const [restrictionMeta, setRestrictionMeta] = useState<RestrictionMetaResponse[]>([])

  useEffect(() => {
    if (!user) return
    let cancelled = false

    const run = async () => {
      try {
        const [meta, restrictions] = await Promise.all([
          getRestrictionMeta().catch(() => [] as RestrictionMetaResponse[]),
          getAdminUserRestrictions(token, user.id),
        ])
        if (cancelled) return
        setRestrictionMeta(meta)
        setUserRestrictions(restrictions)
      } catch (cause) {
        if (cancelled) return
        toast.error(toDisplayError(cause, 'Не удалось загрузить ограничения пользователя.'))
      }
    }

    void run()
    return () => {
      cancelled = true
    }
  }, [token, user])

  const restrictionMetaMap = useMemo(
    () => new Map(restrictionMeta.map((item) => [item.key, item])),
    [restrictionMeta],
  )

  const availableRestrictionKeys = useMemo(() => {
    const keysFromMeta = restrictionMeta.map((item) => item.key)
    const keysFromUser = userRestrictions.map((item) => item.key)
    return Array.from(new Set([...keysFromMeta, ...keysFromUser]))
  }, [restrictionMeta, userRestrictions])

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

  const activateUser = async () => {
    if (!user || isUpdatingActivity) return
    setIsUpdatingActivity(true)
    try {
      const updated = await activateAdminUser(token, user.id)
      onUserChange(updated)
      toast.success('Пользователь активирован.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось активировать пользователя.'))
    } finally {
      setIsUpdatingActivity(false)
    }
  }

  const deactivateUser = async () => {
    if (!user || isUpdatingActivity) return
    const reason = restrictionReason.trim()
    if (!reason) {
      toast.error('Укажите причину деактивации.')
      return
    }

    setIsUpdatingActivity(true)
    try {
      const updated = await deactivateAdminUser(token, user.id, { reason })
      onUserChange(updated)
      toast.success('Пользователь деактивирован.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось деактивировать пользователя.'))
    } finally {
      setIsUpdatingActivity(false)
    }
  }

  const grantRestriction = async (restrictionKey: string) => {
    if (!user || isUpdatingRestrictions) return
    setIsUpdatingRestrictions(true)
    try {
      const updated = await grantAdminUserRestriction(token, user.id, restrictionKey, {
        reason: restrictionReason.trim() || null,
      })
      setUserRestrictions(updated)
      toast.success(`Ограничение "${restrictionKey}" выдано.`)
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось выдать ограничение.'))
    } finally {
      setIsUpdatingRestrictions(false)
    }
  }

  const revokeRestriction = async (restrictionKey: string) => {
    if (!user || isUpdatingRestrictions) return
    setIsUpdatingRestrictions(true)
    try {
      const updated = await revokeAdminUserRestriction(token, user.id, restrictionKey, {
        reason: restrictionReason.trim() || null,
      })
      setUserRestrictions(updated)
      toast.success(`Ограничение "${restrictionKey}" снято.`)
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось снять ограничение.'))
    } finally {
      setIsUpdatingRestrictions(false)
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
        {!user ? (
          <LoadingState title="Загружаем профиль игрока" />
        ) : (
          <>
            <div className="admin-user-headbar">
              <div className="admin-user-headbar-meta">
                <span className={`ui-badge ${user.isSuperuser ? 'ui-badge-secondary' : 'ui-badge-neutral'}`}>
                  {user.isSuperuser ? 'Superuser' : 'Player'}
                </span>
              </div>

              <div className="admin-user-headbar-actions">
                <button
                  className="btn btn-sm"
                  type="button"
                  disabled={isUpdatingSuperuser || user.isSuperuser}
                  onClick={() => void grantSuperuser()}
                >
                  Выдать Superuser
                </button>
                <button
                  className="btn btn-sm danger"
                  type="button"
                  disabled={isUpdatingSuperuser || !user.isSuperuser}
                  onClick={() => void revokeSuperuser()}
                >
                  Снять Superuser
                </button>
              </div>
            </div>

            <div className="admin-user-layout">
              <aside className="admin-user-side">
                <div className="admin-row-user">
                  {user.avatarUrl ? (
                    <img src={user.avatarUrl} alt={user.username} className="admin-avatar admin-avatar-lg" />
                  ) : (
                    <span className="ui-avatar">{initials(user.username)}</span>
                  )}
                  <div className="admin-row-user-text">
                    <h2 className="card-title admin-user-name">{user.username}</h2>
                    <small>{userId}</small>
                  </div>
                </div>

                <div className="admin-user-skin-3d">
                  <SkinViewer3D skinUrl={buildSkinUrl(user.id, skinVersion)} width={280} height={380} />
                </div>

                <button
                  className="btn danger admin-user-skin-action"
                  type="button"
                  disabled={isDeletingSkin}
                  onClick={() => void deleteSkin()}
                >
                  {isDeletingSkin ? 'Удаляем...' : 'Удалить скин'}
                </button>
              </aside>

              <dl className="admin-kv">
                <div><dt>Email</dt><dd>{user.email ?? 'Не указан'}</dd></div>
                <div><dt>Discord ID</dt><dd>{user.discordId}</dd></div>
                <div><dt>Последний вход</dt><dd>{formatDateTime(user.lastLoginAt)}</dd></div>
                <div><dt>Создан</dt><dd>{formatDateTime(user.createdAt)}</dd></div>
                <div>
                  <dt>Статус</dt>
                  <dd>
                    <span className={`ui-badge ${user.isActive ? 'ui-badge-success' : 'ui-badge-warning'}`}>
                      {user.isActive ? 'Активен' : 'Отключен'}
                    </span>
                  </dd>
                </div>
                <div><dt>Причина деактивации</dt><dd>{user.deactivationReason ?? '—'}</dd></div>
                <div>
                  <dt>Сквад</dt>
                  <dd>
                    {userSquad === undefined ? (
                      'Поиск...'
                    ) : userSquad ? (
                      <button type="button" className="btn btn-sm" onClick={() => pushUrl(adminSquadPath(userSquad.id))}>
                        {userSquad.name}
                      </button>
                    ) : (
                      'Не состоит в скваде'
                    )}
                  </dd>
                </div>
              </dl>
            </div>

            <section className="admin-card-subsection">
              <h3 className="card-title">Ограничения пользователя</h3>

              <div className="admin-restriction-toolbar">
                <input
                  className="ui-input"
                  value={restrictionReason}
                  onChange={(event) => setRestrictionReason(event.target.value)}
                  placeholder="Причина (для деактивации обязательно)"
                />
                <button
                  type="button"
                  className="btn btn-sm"
                  disabled={isUpdatingActivity || user.isActive}
                  onClick={() => void activateUser()}
                >
                  Активировать
                </button>
                <button
                  type="button"
                  className="btn btn-sm danger"
                  disabled={isUpdatingActivity || !user.isActive}
                  onClick={() => void deactivateUser()}
                >
                  Деактивировать
                </button>
              </div>

              <div className="admin-restriction-list">
                {availableRestrictionKeys.length === 0 ? (
                  <p className="admin-inline-muted">Справочник ограничений пуст.</p>
                ) : (
                  availableRestrictionKeys.map((key) => {
                    const active = userRestrictions.some((item) => item.key === key)
                    const meta = restrictionMetaMap.get(key)
                    return (
                      <article key={key} className="admin-restriction-item">
                        <div className="admin-restriction-item-main">
                          <strong>{meta?.locale?.en?.title ?? key}</strong>
                          <small>{meta?.locale?.en?.description ?? key}</small>
                        </div>
                        <div className="admin-restriction-item-actions">
                          <span className={`ui-badge ${active ? 'ui-badge-warning' : 'ui-badge-neutral'}`}>
                            {active ? 'Выдано' : 'Нет'}
                          </span>
                          <button
                            type="button"
                            className="btn btn-sm"
                            disabled={isUpdatingRestrictions || active}
                            onClick={() => void grantRestriction(key)}
                          >
                            Выдать
                          </button>
                          <button
                            type="button"
                            className="btn btn-sm danger"
                            disabled={isUpdatingRestrictions || !active}
                            onClick={() => void revokeRestriction(key)}
                          >
                            Снять
                          </button>
                        </div>
                      </article>
                    )
                  })
                )}
              </div>
            </section>
          </>
        )}
      </section>
    </div>
  )
}
