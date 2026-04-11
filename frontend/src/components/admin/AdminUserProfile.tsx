import { useState } from 'react'
import toast from 'react-hot-toast'
import {
  deleteAdminUserSkin,
  grantAdminUserSuperuser,
  revokeAdminUserSuperuser,
  type AdminSquadResponse,
  type AdminUserResponse,
} from '../../api/admin'
import { toDisplayError } from '../../api/http'
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
          </>
        )}
      </section>
    </div>
  )
}
