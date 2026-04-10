import { useMemo, useState } from 'react'
import toast from 'react-hot-toast'
import { createSquadInvite, revokeSquadInvite, type SquadInviteResponse, type UserSearchItemResponse } from '../../../api/squads'
import { toDisplayError } from '../../../api/http'
import { useSquadUserSearch } from '../hooks/useSquadUserSearch'
import { formatDateTime, renderUserAvatar } from '../lib/format'
import type { SquadSectionProps } from '../types'

export default function LeaderInviteCard({ authToken, data, onChanged }: SquadSectionProps) {
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedUser, setSelectedUser] = useState<UserSearchItemResponse | null>(null)
  const [latestInvite, setLatestInvite] = useState<SquadInviteResponse | null>(null)
  const [isSubmitting, setIsSubmitting] = useState(false)
  const { items, isLoading, error } = useSquadUserSearch(authToken, searchQuery)

  const availableItems = useMemo(
    () => items.filter((item) => !data.squadMembers.some((member) => member.id === item.id)),
    [data.squadMembers, items],
  )

  const onInvite = async () => {
    if (!data.squad || !selectedUser) return

    setIsSubmitting(true)
    const request = createSquadInvite(authToken, data.squad.id, selectedUser.id)
    toast.promise(request, {
      loading: 'Создаем инвайт...',
      success: 'Инвайт отправлен.',
      error: (cause) => toDisplayError(cause, 'Не удалось создать инвайт.'),
    })

    try {
      const invite = await request
      setLatestInvite(invite)
      setSearchQuery('')
      setSelectedUser(null)
      await onChanged()
    } finally {
      setIsSubmitting(false)
    }
  }

  const onRevokeLatestInvite = async () => {
    if (!latestInvite) return

    const request = revokeSquadInvite(authToken, latestInvite.id)
    toast.promise(request, {
      loading: 'Отзываем инвайт...',
      success: 'Инвайт отозван.',
      error: (cause) => toDisplayError(cause, 'Не удалось отозвать инвайт.'),
    })

    await request
    setLatestInvite(null)
  }

  return (
    <section className="card profile-panel">
      <div className="ui-card-header">
        <h2 className="card-title">Инвайты</h2>
      </div>
      <div className="profile-form">
        <div className="ui-field">
          <label className="ui-label" htmlFor="squad-user-search">Поиск игрока</label>
          <input
            id="squad-user-search"
            className="ui-input"
            value={searchQuery}
            onChange={(event) => {
              setSearchQuery(event.target.value)
              setSelectedUser(null)
            }}
            placeholder="Введите минимум 3 символа"
          />
          <div className="ui-hint">Для инвайта используйте поиск по нику.</div>
        </div>

        {searchQuery.trim().length >= 3 ? (
          <div className="profile-search-results">
            {isLoading ? <div className="profile-subtle">Ищем игроков...</div> : null}
            {error ? <div className="profile-subtle">{error}</div> : null}
            {!isLoading && !error && !availableItems.length ? <div className="profile-subtle">Ничего не найдено.</div> : null}
            {!isLoading && !error ? availableItems.map((item) => (
              <button
                key={item.id}
                type="button"
                className={`profile-search-item ${selectedUser?.id === item.id ? 'is-active' : ''}`}
                onClick={() => setSelectedUser(item)}
              >
                <span className="profile-search-item__main">
                  {renderUserAvatar(item)}
                  <span>{item.username}</span>
                </span>
              </button>
            )) : null}
          </div>
        ) : null}

        <div className="profile-actions">
          <button className="btn primary" type="button" disabled={!selectedUser || isSubmitting} onClick={() => void onInvite()}>
            {isSubmitting ? 'Отправка...' : 'Отправить инвайт'}
          </button>
        </div>
      </div>

      {latestInvite ? (
        <>
          <div className="ui-divider" />
          <div className="profile-stack">
            <div className="profile-inline-card">
              <strong>Последний инвайт</strong>
              <span className="profile-subtle">{latestInvite.squadName}</span>
            </div>
            <div className="profile-inline-card">
              <strong>Истекает</strong>
              <span className="profile-subtle">{formatDateTime(latestInvite.expiresAt)}</span>
            </div>
            <div className="profile-actions">
              <button className="btn" type="button" onClick={() => void onRevokeLatestInvite()}>
                Отозвать последний инвайт
              </button>
            </div>
          </div>
        </>
      ) : null}
    </section>
  )
}
