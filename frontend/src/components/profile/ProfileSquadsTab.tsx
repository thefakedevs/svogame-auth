import { useEffect, useMemo, useState } from 'react'
import toast from 'react-hot-toast'
import {
  acceptSquadInvite,
  createSquad,
  createSquadInvite,
  declineSquadInvite,
  deleteSquad,
  kickSquadMember,
  leaveSquad,
  patchSquad,
  revokeSquadInvite,
  searchUsers,
  type SquadInviteResponse,
  type UserSearchItemResponse,
} from '../../api/profile'
import { toDisplayError } from '../../api/http'
import type { ProfileDashboardData } from './types'

const SQUAD_VIEW_FADE_OUT_MS = 120
const SQUAD_VIEW_FADE_IN_MS = 180

type SquadViewMode = 'no_squad' | 'leader' | 'member'
type SquadViewTransition = 'idle' | 'fade-out' | 'fade-in'

function resolveSquadViewMode(data: ProfileDashboardData): SquadViewMode {
  if (!data.squad) return 'no_squad'
  return data.squad.leaderUserId === data.user.id ? 'leader' : 'member'
}

const dateTimeFormatter = new Intl.DateTimeFormat('ru-RU', {
  day: '2-digit',
  month: 'short',
  year: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
})

const dateFormatter = new Intl.DateTimeFormat('ru-RU', {
  day: '2-digit',
  month: 'short',
  year: 'numeric',
})

function initials(value: string) {
  return value.slice(0, 2).toUpperCase()
}

function formatDate(value?: string | null) {
  if (!value) return 'Нет данных'
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : dateFormatter.format(date)
}

function formatDateTime(value?: string | null) {
  if (!value) return 'Нет данных'
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : dateTimeFormatter.format(date)
}

function renderUserAvatar(user: { username: string; avatarUrl?: string | null }) {
  return user.avatarUrl ? (
    <img className="profile-member-avatar profile-member-avatar--sm" src={user.avatarUrl} alt={user.username} />
  ) : (
    <span className="profile-member-avatar profile-member-avatar--sm">{initials(user.username)}</span>
  )
}

function useUserSearch(authToken: string | null, query: string) {
  const [items, setItems] = useState<UserSearchItemResponse[]>([])
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState('')

  useEffect(() => {
    if (!authToken) {
      setItems([])
      setError('')
      return
    }

    const trimmed = query.trim()
    if (trimmed.length < 3) {
      setItems([])
      setError('')
      return
    }

    let cancelled = false
    const timerId = window.setTimeout(async () => {
      try {
        setIsLoading(true)
        setError('')
        const result = await searchUsers(authToken, trimmed, 10)
        if (cancelled) return
        setItems(result)
      } catch (cause) {
        if (cancelled) return
        setItems([])
        setError(toDisplayError(cause, 'Не удалось найти игроков.'))
      } finally {
        if (!cancelled) {
          setIsLoading(false)
        }
      }
    }, 250)

    return () => {
      cancelled = true
      window.clearTimeout(timerId)
    }
  }, [authToken, query])

  return { items, isLoading, error }
}

function SquadInfoCard({ data, isLeader }: { data: ProfileDashboardData; isLeader: boolean }) {
  return (
    <section className="card profile-panel">
      <div className="ui-card-header">
        <h2 className="card-title">{isLeader ? 'Настройки сквада' : 'Информация о скваде'}</h2>
      </div>
      <div className="profile-stack">
        <div className="profile-inline-card">
          <strong>Название</strong>
          <span className="profile-subtle">{data.squad?.name ?? 'Нет данных'}</span>
        </div>
        <div className="profile-inline-card">
          <strong>Создан</strong>
          <span className="profile-subtle">{formatDate(data.squad?.createdAt)}</span>
        </div>
        <div className="profile-inline-card">
          <strong>Участников</strong>
          <span className="profile-subtle">{data.squad?.memberCount ?? 0} / {data.squad?.maxMembers ?? data.squadConfig.maxMembers}</span>
        </div>
        <div className="profile-inline-card">
          <strong>Лидер</strong>
          <span className="profile-subtle">
            {data.squadMembers.find((member) => member.isLeader)?.username ?? 'Нет данных'}
          </span>
        </div>
      </div>
    </section>
  )
}

function SquadMembersCard({
  data,
  isLeader,
  authToken,
  onChanged,
}: {
  data: ProfileDashboardData
  isLeader: boolean
  authToken: string
  onChanged: () => Promise<void>
}) {
  const [processingMemberId, setProcessingMemberId] = useState<string | null>(null)

  const onKick = async (userId: string) => {
    if (!data.squad) return

    setProcessingMemberId(userId)
    const request = kickSquadMember(authToken, data.squad.id, userId)
    toast.promise(request, {
      loading: 'Исключаем участника...',
      success: 'Участник исключен из сквада.',
      error: (cause) => toDisplayError(cause, 'Не удалось исключить участника.'),
    })

    try {
      await request
      await onChanged()
    } finally {
      setProcessingMemberId(null)
    }
  }

  return (
    <section className="card profile-panel">
      <div className="ui-card-header">
        <h2 className="card-title">Участники</h2>
        <span className="ui-badge ui-badge-neutral">{data.squadMembers.length} / {data.squad?.maxMembers ?? data.squadConfig.maxMembers}</span>
      </div>
      <div className="profile-member-list">
        {data.squadMembers.map((member) => (
          <div key={member.id} className="profile-member">
            <div className="profile-member-avatar">
              {member.avatarUrl ? <img src={member.avatarUrl} alt={member.username} /> : <span>{initials(member.username)}</span>}
            </div>
            <div className="profile-member-copy">
              <strong>{member.username}</strong>
              <span className="profile-subtle">{member.isLeader ? 'Лидер' : 'Участник'}</span>
            </div>
            {isLeader && !member.isLeader ? (
              <button
                className="btn btn-sm"
                type="button"
                disabled={processingMemberId === member.id}
                onClick={() => void onKick(member.id)}
              >
                {processingMemberId === member.id ? 'Исключение...' : 'Кикнуть'}
              </button>
            ) : null}
          </div>
        ))}
      </div>
    </section>
  )
}

function LeaderInviteCard({
  authToken,
  data,
  onChanged,
}: {
  authToken: string
  data: ProfileDashboardData
  onChanged: () => Promise<void>
}) {
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedUser, setSelectedUser] = useState<UserSearchItemResponse | null>(null)
  const [latestInvite, setLatestInvite] = useState<SquadInviteResponse | null>(null)
  const [isSubmitting, setIsSubmitting] = useState(false)
  const { items, isLoading, error } = useUserSearch(authToken, searchQuery)

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
          <button
            className="btn primary"
            type="button"
            disabled={!selectedUser || isSubmitting}
            onClick={() => void onInvite()}
          >
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

function LeaderManagementCard({
  authToken,
  data,
  onChanged,
}: {
  authToken: string
  data: ProfileDashboardData
  onChanged: () => Promise<void>
}) {
  const [nameDraft, setNameDraft] = useState(data.squad?.name ?? '')
  const [isSaving, setIsSaving] = useState(false)
  const [isDeleting, setIsDeleting] = useState(false)

  useEffect(() => {
    setNameDraft(data.squad?.name ?? '')
  }, [data.squad?.name])

  const onSave = async () => {
    if (!data.squad) return
    const trimmed = nameDraft.trim()
    if (!trimmed || trimmed === data.squad.name) return

    setIsSaving(true)
    const request = patchSquad(authToken, data.squad.id, trimmed)
    toast.promise(request, {
      loading: 'Сохраняем сквад...',
      success: 'Настройки сквада обновлены.',
      error: (cause) => toDisplayError(cause, 'Не удалось обновить сквад.'),
    })

    try {
      await request
      await onChanged()
    } finally {
      setIsSaving(false)
    }
  }

  const onDelete = async () => {
    if (!data.squad) return
    setIsDeleting(true)
    const request = deleteSquad(authToken, data.squad.id)
    toast.promise(request, {
      loading: 'Распускаем сквад...',
      success: 'Сквад удален.',
      error: (cause) => toDisplayError(cause, 'Не удалось удалить сквад.'),
    })

    try {
      await request
      await onChanged()
    } finally {
      setIsDeleting(false)
    }
  }

  return (
    <section className="card profile-panel">
      <div className="ui-card-header">
        <h2 className="card-title">Управление сквадом</h2>
      </div>
      <div className="profile-form">
        <div className="ui-field">
          <label className="ui-label" htmlFor="squad-name">Название</label>
          <input
            id="squad-name"
            className="ui-input"
            value={nameDraft}
            onChange={(event) => setNameDraft(event.target.value)}
            placeholder="Введите название сквада"
          />
          <div className="ui-hint">
            Лимиты: {data.squadConfig.nameMinChars}–{data.squadConfig.nameMaxChars} символов.
          </div>
        </div>
        <div className="profile-actions">
          <button className="btn primary" type="button" disabled={isSaving} onClick={() => void onSave()}>
            {isSaving ? 'Сохранение...' : 'Сохранить название'}
          </button>
          <button className="btn danger" type="button" disabled={isDeleting} onClick={() => void onDelete()}>
            {isDeleting ? 'Удаление...' : 'Удалить сквад'}
          </button>
        </div>
      </div>
    </section>
  )
}

function MemberActionsCard({
  authToken,
  squadId,
  onChanged,
}: {
  authToken: string
  squadId: string
  onChanged: () => Promise<void>
}) {
  const [isLeaving, setIsLeaving] = useState(false)

  const onLeave = async () => {
    setIsLeaving(true)
    const request = leaveSquad(authToken, squadId)
    toast.promise(request, {
      loading: 'Выходим из сквада...',
      success: 'Вы покинули сквад.',
      error: (cause) => toDisplayError(cause, 'Не удалось покинуть сквад.'),
    })

    try {
      await request
      await onChanged()
    } finally {
      setIsLeaving(false)
    }
  }

  return (
    <section className="card profile-panel">
      <div className="ui-card-header">
        <h2 className="card-title">Действия</h2>
      </div>
      <div className="profile-actions">
        <button className="btn danger" type="button" disabled={isLeaving} onClick={() => void onLeave()}>
          {isLeaving ? 'Выход...' : 'Покинуть сквад'}
        </button>
      </div>
    </section>
  )
}

function NoSquadState({
  authToken,
  data,
  onChanged,
}: {
  authToken: string
  data: ProfileDashboardData
  onChanged: () => Promise<void>
}) {
  const [nameDraft, setNameDraft] = useState('')
  const [isCreating, setIsCreating] = useState(false)
  const [processingInviteId, setProcessingInviteId] = useState<string | null>(null)

  const onCreate = async () => {
    const trimmed = nameDraft.trim()
    if (!trimmed) return

    setIsCreating(true)
    const request = createSquad(authToken, trimmed)
    toast.promise(request, {
      loading: 'Создаем сквад...',
      success: 'Сквад создан.',
      error: (cause) => toDisplayError(cause, 'Не удалось создать сквад.'),
    })

    try {
      await request
      setNameDraft('')
      await onChanged()
    } finally {
      setIsCreating(false)
    }
  }

  const onAcceptInvite = async (inviteId: string) => {
    setProcessingInviteId(inviteId)
    const request = acceptSquadInvite(authToken, inviteId)
    toast.promise(request, {
      loading: 'Принимаем инвайт...',
      success: 'Вы вступили в сквад.',
      error: (cause) => toDisplayError(cause, 'Не удалось принять инвайт.'),
    })

    try {
      await request
      await onChanged()
    } finally {
      setProcessingInviteId(null)
    }
  }

  const onDeclineInvite = async (inviteId: string) => {
    setProcessingInviteId(inviteId)
    const request = declineSquadInvite(authToken, inviteId)
    toast.promise(request, {
      loading: 'Отклоняем инвайт...',
      success: 'Инвайт отклонен.',
      error: (cause) => toDisplayError(cause, 'Не удалось отклонить инвайт.'),
    })

    try {
      await request
      await onChanged()
    } finally {
      setProcessingInviteId(null)
    }
  }

  return (
    <div className="profile-split">
      <section className="card profile-panel">
        <div className="ui-card-header">
          <h2 className="card-title">Создать сквад</h2>
        </div>
        <div className="profile-form">
          <div className="ui-field">
            <label className="ui-label" htmlFor="create-squad-name">Название</label>
            <input
              id="create-squad-name"
              className="ui-input"
              value={nameDraft}
              onChange={(event) => setNameDraft(event.target.value)}
              placeholder="Введите название сквада"
            />
            <div className="ui-hint">
              Лимиты: {data.squadConfig.nameMinChars}–{data.squadConfig.nameMaxChars} символов, инвайт действует {data.squadConfig.inviteTtlHours} часа.
            </div>
          </div>
          <div className="profile-actions">
            <button className="btn primary" type="button" disabled={isCreating} onClick={() => void onCreate()}>
              {isCreating ? 'Создание...' : 'Создать сквад'}
            </button>
          </div>
        </div>
      </section>

      <section className="card profile-panel">
        <div className="ui-card-header">
          <h2 className="card-title">Мои приглашения</h2>
          <span className="ui-badge ui-badge-warning">{data.squadInvites.length}</span>
        </div>
        {data.squadInvites.length ? (
          <div className="profile-stack">
            {data.squadInvites.map((invite) => (
              <div key={invite.id} className="profile-invite-card">
                <div className="profile-stack">
                  <strong>{invite.squadName}</strong>
                  <span className="profile-subtle">Истекает {formatDateTime(invite.expiresAt)}</span>
                </div>
                <div className="profile-actions">
                  <button
                    className="btn primary"
                    type="button"
                    disabled={processingInviteId === invite.id}
                    onClick={() => void onAcceptInvite(invite.id)}
                  >
                    {processingInviteId === invite.id ? 'Обработка...' : 'Принять'}
                  </button>
                  <button
                    className="btn"
                    type="button"
                    disabled={processingInviteId === invite.id}
                    onClick={() => void onDeclineInvite(invite.id)}
                  >
                    Отклонить
                  </button>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="profile-empty profile-empty--wide">
            <span className="ui-badge ui-badge-neutral">Нет приглашений</span>
            <p>Когда вас пригласят в сквад, приглашения появятся здесь.</p>
          </div>
        )}
      </section>
    </div>
  )
}

export default function ProfileSquadsTab({
  data,
  authToken,
  onChanged,
}: {
  data: ProfileDashboardData
  authToken: string | null
  onChanged: () => Promise<void>
}) {
  if (!authToken) {
    return null
  }

  const nextMode = resolveSquadViewMode(data)
  const [displayMode, setDisplayMode] = useState<SquadViewMode>(nextMode)
  const [transition, setTransition] = useState<SquadViewTransition>('idle')
  const [displayData, setDisplayData] = useState<ProfileDashboardData>(data)

  useEffect(() => {
    if (nextMode === displayMode) {
      if (transition === 'idle') {
        setDisplayData(data)
      }
      return
    }

    setTransition('fade-out')

    const switchTimerId = window.setTimeout(() => {
      setDisplayMode(nextMode)
      setDisplayData(data)
      setTransition('fade-in')
    }, SQUAD_VIEW_FADE_OUT_MS)

    const endTimerId = window.setTimeout(() => {
      setTransition('idle')
    }, SQUAD_VIEW_FADE_OUT_MS + SQUAD_VIEW_FADE_IN_MS)

    return () => {
      window.clearTimeout(switchTimerId)
      window.clearTimeout(endTimerId)
    }
  }, [data, displayMode, nextMode, transition])

  const stageClass =
    transition === 'fade-out'
      ? 'is-fading-out'
      : transition === 'fade-in'
        ? 'is-fading-in'
        : ''

  if (displayMode === 'no_squad') {
    return (
      <div className={`profile-squad-stage ${stageClass}`}>
        <NoSquadState authToken={authToken} data={displayData} onChanged={onChanged} />
      </div>
    )
  }

  if (!displayData.squad) {
    return null
  }

  if (displayMode === 'leader') {
    return (
      <div className={`profile-squad-stage ${stageClass}`}>
        <div className="profile-split">
          <div className="profile-stack">
            <SquadInfoCard data={displayData} isLeader />
            <LeaderManagementCard authToken={authToken} data={displayData} onChanged={onChanged} />
            <LeaderInviteCard authToken={authToken} data={displayData} onChanged={onChanged} />
          </div>
          <SquadMembersCard data={displayData} isLeader authToken={authToken} onChanged={onChanged} />
        </div>
      </div>
    )
  }

  return (
    <div className={`profile-squad-stage ${stageClass}`}>
      <div className="profile-split">
        <div className="profile-stack">
          <SquadInfoCard data={displayData} isLeader={false} />
          <MemberActionsCard authToken={authToken} squadId={displayData.squad.id} onChanged={onChanged} />
        </div>
        <SquadMembersCard data={displayData} isLeader={false} authToken={authToken} onChanged={onChanged} />
      </div>
    </div>
  )
}
