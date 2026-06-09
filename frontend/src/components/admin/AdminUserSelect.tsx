import { useEffect, useMemo, useState } from 'react'
import {
  getAdminUser,
  listAdminUsers,
  type AdminUserResponse,
} from '../../api/admin'
import { toDisplayError } from '../../api/http'
import { searchUsers } from '../../api/squads'

type AdminUserSelectProps = {
  token: string
  value: string
  onChange: (userId: string, user: AdminUserResponse | null) => void
  disabled?: boolean
  placeholder?: string
  allowEmpty?: boolean
}

function userMeta(user: AdminUserResponse) {
  return [user.id, user.email, user.discordId].filter(Boolean).join(' · ')
}

export default function AdminUserSelect({
  token,
  value,
  onChange,
  disabled = false,
  placeholder = 'Ник, email или UUID',
  allowEmpty = true,
}: AdminUserSelectProps) {
  const [query, setQuery] = useState('')
  const [items, setItems] = useState<AdminUserResponse[]>([])
  const [selectedUser, setSelectedUser] = useState<AdminUserResponse | null>(null)
  const [isOpen, setIsOpen] = useState(false)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState('')

  useEffect(() => {
    if (!value) {
      setSelectedUser(null)
      return
    }

    if (selectedUser?.id === value) return

    let cancelled = false
    getAdminUser(token, value)
      .then((user) => {
        if (!cancelled) setSelectedUser(user)
      })
      .catch(() => {
        if (!cancelled) setSelectedUser(null)
      })

    return () => {
      cancelled = true
    }
  }, [selectedUser, token, value])

  useEffect(() => {
    if (disabled) return

    let cancelled = false
    const timerId = window.setTimeout(async () => {
      setIsLoading(true)
      setError('')
      try {
        const trimmed = query.trim()
        if (trimmed.length > 0) {
          const response = await searchUsers(token, trimmed, 20)
          if (!cancelled) {
            setItems(response.map((user) => ({
              id: user.id,
              username: user.username,
              avatarUrl: user.avatarUrl,
              discordId: '',
              email: null,
              authEpoch: 0,
              isActive: true,
              isSuperuser: false,
              lastLoginAt: '',
              createdAt: '',
              deactivationReason: null,
            })))
          }
          return
        }

        const response = await listAdminUsers(token, {
          page: 1,
          perPage: 20,
        })
        if (!cancelled) setItems(response.items)
      } catch (cause) {
        if (!cancelled) {
          setItems([])
          setError(toDisplayError(cause, 'Не удалось найти игроков.'))
        }
      } finally {
        if (!cancelled) setIsLoading(false)
      }
    }, 250)

    return () => {
      cancelled = true
      window.clearTimeout(timerId)
    }
  }, [disabled, query, token])

  const visibleItems = useMemo(() => {
    if (!selectedUser) return items
    return items.some((item) => item.id === selectedUser.id) ? items : [selectedUser, ...items]
  }, [items, selectedUser])

  const selectUser = (user: AdminUserResponse) => {
    setSelectedUser(user)
    onChange(user.id, user)
    setQuery('')
    setIsOpen(false)
  }

  return (
    <div className="admin-picker admin-user-picker">
      {selectedUser ? (
        <div className="admin-picker-selected">
          <span className="admin-user-picker-avatar">
            {selectedUser.avatarUrl ? <img src={selectedUser.avatarUrl} alt="" loading="lazy" /> : selectedUser.username.slice(0, 1).toUpperCase()}
          </span>
          <span className="admin-picker-selected__text">
            <strong>{selectedUser.username}</strong>
            <small>{userMeta(selectedUser)}</small>
          </span>
          {allowEmpty ? (
            <button type="button" className="btn btn-sm" disabled={disabled} onClick={() => onChange('', null)}>
              Сбросить
            </button>
          ) : null}
        </div>
      ) : null}
      {!selectedUser ? (
        <input
          className="ui-input"
          value={query}
          disabled={disabled}
          onFocus={() => setIsOpen(true)}
          onChange={(event) => {
            setQuery(event.target.value)
            setIsOpen(true)
          }}
          placeholder={placeholder}
        />
      ) : null}
      {isOpen && !disabled ? (
        <div className="admin-picker-menu">
          {isLoading ? <p className="admin-inline-muted admin-picker-empty">Ищем игроков...</p> : null}
          {error ? <p className="admin-inline-muted admin-picker-empty">{error}</p> : null}
          {!isLoading && !error ? visibleItems.map((user) => (
            <button
              key={user.id}
              type="button"
              className={`admin-picker-option ${user.id === value ? 'is-active' : ''}`}
              onMouseDown={(event) => {
                event.preventDefault()
                event.stopPropagation()
                selectUser(user)
              }}
              onClick={() => selectUser(user)}
            >
              <span className="admin-user-picker-avatar">
                {user.avatarUrl ? <img src={user.avatarUrl} alt="" loading="lazy" /> : user.username.slice(0, 1).toUpperCase()}
              </span>
              <span className="admin-picker-option__text">
                <strong>{user.username}</strong>
                <small>{userMeta(user)}</small>
              </span>
            </button>
          )) : null}
          {!isLoading && !error && visibleItems.length === 0 ? <p className="admin-inline-muted admin-picker-empty">Игроки не найдены.</p> : null}
        </div>
      ) : null}
    </div>
  )
}
