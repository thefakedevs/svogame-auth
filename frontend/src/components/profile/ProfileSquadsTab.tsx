import { useEffect, useMemo, useState } from 'react'
import { createPortal } from 'react-dom'
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
  uploadSquadImage,
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

function sanitizeSquadName(value: string) {
  return value.replace(/[^A-Za-zА-Яа-яЁё-]/g, '')
}

function getModalPortalTarget() {
  if (typeof document === 'undefined') return null
  return document.querySelector('.ui-kit-page.app-shell')
}

function hasInvalidSquadNameBoundary(value: string) {
  return value.startsWith('-') || value.endsWith('-')
}

function isInvalidSquadNameLength(value: string) {
  return value.length < 4 || value.length > 16
}

function formatImageLimit(bytes: number) {
  const megabytes = bytes / (1024 * 1024)
  if (Number.isInteger(megabytes)) return `${megabytes} МБ`
  return `${megabytes.toFixed(1).replace('.', ',')} МБ`
}

function getSquadNameRegex(pattern: string) {
  return new RegExp(pattern, 'u')
}

function hasInvalidSquadNameByConfig(value: string, config: ProfileDashboardData['squadConfig']) {
  if (!value) return false

  return (
    value.length < config.nameMinChars ||
    value.length > config.nameMaxChars ||
    !getSquadNameRegex(config.nameRegex).test(value)
  )
}

async function validateSquadImageByConfig(file: File, config: ProfileDashboardData['squadConfig']) {
  if (file.size > config.imageMaxBytes) {
    return `Файл должен быть не больше ${formatImageLimit(config.imageMaxBytes)}.`
  }

  const objectUrl = URL.createObjectURL(file)

  try {
    const { width, height } = await new Promise<{ width: number; height: number }>((resolve, reject) => {
      const image = new Image()
      image.onload = () => resolve({ width: image.naturalWidth, height: image.naturalHeight })
      image.onerror = () => reject(new Error('Не удалось прочитать изображение.'))
      image.src = objectUrl
    })

    if (width > config.imageMaxWidth || height > config.imageMaxHeight) {
      return `Изображение должно быть не больше ${config.imageMaxWidth}x${config.imageMaxHeight}px.`
    }

    return null
  } finally {
    URL.revokeObjectURL(objectUrl)
  }
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

function SquadInfoCard({
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
  const [isUploadingImage, setIsUploadingImage] = useState(false)
  const [isDragActive, setIsDragActive] = useState(false)
  const [nameDraft, setNameDraft] = useState(data.squad?.name ?? '')
  const [hasInvalidNameInput, setHasInvalidNameInput] = useState(false)
  const [isEditingName, setIsEditingName] = useState(false)
  const [isSavingName, setIsSavingName] = useState(false)
  const [isDeleteModalOpen, setIsDeleteModalOpen] = useState(false)
  const [isDeletingSquad, setIsDeletingSquad] = useState(false)
  const squadImageUrl = data.squad?.imageUrl ? `${data.squad.imageUrl}?v=${encodeURIComponent(data.squad.updatedAt)}` : null
  const isAvatarUploadDisabled = !isLeader || isUploadingImage
  const squadAvatarInputId = `squad-avatar-upload-${data.squad?.id ?? 'current'}`
  const squadNamePattern = data.squadConfig.nameRegex
  const squadImageHint = `до ${data.squadConfig.imageMaxWidth}x${data.squadConfig.imageMaxHeight} и ${formatImageLimit(data.squadConfig.imageMaxBytes)}`
  const avatarActionLabel = isUploadingImage
    ? 'Загрузка...'
    : isLeader
      ? `Аватар ${squadImageHint}`
      : 'Недоступно: только лидер'

  useEffect(() => {
    setNameDraft(data.squad?.name ?? '')
    setHasInvalidNameInput(false)
  }, [data.squad?.name])

  const onNameDraftChange = (value: string) => {
    setNameDraft(value)
    setHasInvalidNameInput(hasInvalidSquadNameByConfig(value.trim(), data.squadConfig))
  }

  const onImageSelected = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0]
    if (!file || !data.squad) return

    const imageValidationError = await validateSquadImageByConfig(file, data.squadConfig)
    if (imageValidationError) {
      toast.error(imageValidationError)
      event.target.value = ''
      return
    }

    setIsUploadingImage(true)
    const request = uploadSquadImage(authToken, data.squad.id, file)
    toast.promise(request, {
      loading: 'Загружаем аватар сквада...',
      success: 'Аватар сквада обновлен.',
      error: (cause) => toDisplayError(cause, 'Не удалось загрузить аватар сквада.'),
    })

    try {
      await request
      await onChanged()
    } finally {
      setIsUploadingImage(false)
      setIsDragActive(false)
      event.target.value = ''
    }
  }

  const onSaveName = async () => {
    if (!data.squad) return
    const trimmed = nameDraft.trim()
    if (hasInvalidSquadNameByConfig(trimmed, data.squadConfig)) {
      setHasInvalidNameInput(true)
      return
    }
    if (!trimmed || trimmed === data.squad.name) {
      setIsEditingName(false)
      setNameDraft(data.squad.name)
      return
    }

    setIsSavingName(true)
    const request = patchSquad(authToken, data.squad.id, trimmed)
    toast.promise(request, {
      loading: 'Сохраняем сквад...',
      success: 'Настройки сквада обновлены.',
      error: (cause) => toDisplayError(cause, 'Не удалось обновить сквад.'),
    })

    try {
      await request
      await onChanged()
      setIsEditingName(false)
    } finally {
      setIsSavingName(false)
    }
  }

  const onCancelNameEdit = () => {
    setNameDraft(data.squad?.name ?? '')
    setHasInvalidNameInput(false)
    setIsEditingName(false)
  }

  const onDeleteSquad = async () => {
    if (!data.squad) return

    setIsDeletingSquad(true)
    const request = deleteSquad(authToken, data.squad.id)
    toast.promise(request, {
      loading: 'Распускаем сквад...',
      success: 'Сквад удален.',
      error: (cause) => toDisplayError(cause, 'Не удалось удалить сквад.'),
    })

    try {
      await request
      setIsDeleteModalOpen(false)
      await onChanged()
    } finally {
      setIsDeletingSquad(false)
    }
  }

  return (
    <section className="card profile-panel">
      <div className="ui-card-header">
        <h2 className="card-title">{isLeader ? 'Настройки сквада' : 'Информация о скваде'}</h2>
      </div>
      <div className="profile-stack">
        <div className="profile-squad-head-row">
          {isLeader ? (
            <label
              className={`profile-squad-avatar-tile is-clickable ${isDragActive ? 'is-drag-active' : ''}`}
              aria-disabled={isAvatarUploadDisabled}
              aria-label={avatarActionLabel}
              htmlFor={squadAvatarInputId}
              onDragEnter={(event) => {
                if (!event.dataTransfer.types.includes('Files')) return
                setIsDragActive(true)
              }}
              onDragOver={(event) => {
                if (!event.dataTransfer.types.includes('Files')) return
                event.preventDefault()
                event.dataTransfer.dropEffect = 'copy'
                if (!isDragActive) setIsDragActive(true)
              }}
              onDragLeave={(event) => {
                const nextTarget = event.relatedTarget
                if (nextTarget instanceof Node && event.currentTarget.contains(nextTarget)) return
                setIsDragActive(false)
              }}
              onDrop={() => {
                setIsDragActive(false)
              }}
            >
              {squadImageUrl ? (
                <img className="profile-squad-avatar-image" src={squadImageUrl} alt={`Аватар сквада ${data.squad?.name ?? ''}`} />
              ) : (
                <span className="profile-squad-avatar-placeholder" aria-hidden>
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" />
                    <polyline points="17 8 12 3 7 8" />
                    <line x1="12" y1="3" x2="12" y2="15" />
                  </svg>
                  <span className="profile-squad-avatar-placeholder-copy">{squadImageHint}</span>
                </span>
              )}
              <span className="profile-squad-avatar-tooltip" role="tooltip">
                {avatarActionLabel}
              </span>
              <span className="profile-squad-avatar-overlay" aria-hidden>
                Отпустите файл
              </span>
              <input
                id={squadAvatarInputId}
                className="profile-squad-avatar-input"
                type="file"
                accept="image/*"
                disabled={isUploadingImage}
                onChange={(event) => void onImageSelected(event)}
              />
            </label>
          ) : (
            <div className="profile-squad-avatar-tile is-readonly" aria-disabled="true" aria-label={avatarActionLabel}>
              {squadImageUrl ? (
                <img className="profile-squad-avatar-image" src={squadImageUrl} alt={`Аватар сквада ${data.squad?.name ?? ''}`} />
              ) : (
                <span className="profile-squad-avatar-placeholder" aria-hidden>
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" />
                    <polyline points="17 8 12 3 7 8" />
                    <line x1="12" y1="3" x2="12" y2="15" />
                  </svg>
                  <span className="profile-squad-avatar-placeholder-copy">Недоступно</span>
                </span>
              )}
              <span className="profile-squad-avatar-tooltip" role="tooltip">
                {avatarActionLabel}
              </span>
            </div>
          )}
          <div className="profile-squad-name-block">
            {!isEditingName ? (
              <div className="profile-squad-name-row">
                <strong className="profile-squad-name-inline">{data.squad?.name ?? 'Нет данных'}</strong>
                {isLeader ? (
                  <button
                    className="profile-icon-button"
                    type="button"
                    aria-label="Изменить название сквада"
                    onClick={() => setIsEditingName(true)}
                    disabled={isSavingName || isDeletingSquad}
                  >
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <path d="M12 20h9" />
                      <path d="M16.5 3.5a2.12 2.12 0 013 3L7 19l-4 1 1-4 12.5-12.5z" />
                    </svg>
                  </button>
                ) : null}
              </div>
            ) : null}
            {isLeader && isEditingName ? (
              <div className="profile-form">
                <div className={`ui-field ${hasInvalidNameInput ? 'ui-field-error' : ''}`}>
                  <input
                    id="squad-name"
                    className="ui-input"
                    value={nameDraft}
                    onChange={(event) => onNameDraftChange(event.target.value)}
                    placeholder="Введите название сквада"
                    pattern={squadNamePattern}
                    minLength={data.squadConfig.nameMinChars}
                    maxLength={data.squadConfig.nameMaxChars}
                    inputMode="text"
                    autoComplete="off"
                    autoFocus
                  />
                  <div className={`ui-hint ${hasInvalidNameInput ? 'ui-hint-error' : ''}`}>
                    Лимиты: {data.squadConfig.nameMinChars}–{data.squadConfig.nameMaxChars} символов. Разрешены: латиница, кириллица и `-`. Имя не может начинаться или заканчиваться на `-`.
                  </div>
                </div>
                <div className="profile-actions">
                  <button className="btn primary" type="button" disabled={isSavingName} onClick={() => void onSaveName()}>
                    {isSavingName ? 'Сохранение...' : 'Сохранить'}
                  </button>
                  <button className="btn" type="button" disabled={isSavingName} onClick={onCancelNameEdit}>
                    Отменить
                  </button>
                </div>
              </div>
            ) : null}
          </div>
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
        {isLeader ? (
          <div className="profile-actions">
            <button className="btn danger" type="button" disabled={isDeletingSquad} onClick={() => setIsDeleteModalOpen(true)}>
              {isDeletingSquad ? 'Удаление...' : 'Удалить сквад'}
            </button>
          </div>
        ) : null}
      </div>
      {isLeader && isDeleteModalOpen && getModalPortalTarget()
        ? createPortal(
            <div className="ui-modal-backdrop" role="presentation" onClick={() => !isDeletingSquad && setIsDeleteModalOpen(false)}>
              <div
                className="ui-modal"
                role="dialog"
                aria-modal="true"
                aria-labelledby="delete-squad-modal-title"
                onClick={(event) => event.stopPropagation()}
              >
                <div className="ui-modal-header">
                  <h2 id="delete-squad-modal-title" className="ui-modal-title">
                    Удалить сквад?
                  </h2>
                  <button
                    className="ui-modal-close"
                    type="button"
                    aria-label="Закрыть"
                    onClick={() => setIsDeleteModalOpen(false)}
                    disabled={isDeletingSquad}
                  >
                    ×
                  </button>
                </div>
                <div className="ui-modal-body">
                  <p>
                    Сквад <strong>{data.squad?.name ?? ''}</strong> будет удален без возможности восстановления. Подтвердите действие.
                  </p>
                </div>
                <div className="ui-modal-footer">
                  <button className="btn" type="button" onClick={() => setIsDeleteModalOpen(false)} disabled={isDeletingSquad}>
                    Отменить
                  </button>
                  <button className="btn danger" type="button" onClick={() => void onDeleteSquad()} disabled={isDeletingSquad}>
                    {isDeletingSquad ? 'Удаление...' : 'Удалить'}
                  </button>
                </div>
              </div>
            </div>,
            getModalPortalTarget()!,
          )
        : null}
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
  const [memberToKick, setMemberToKick] = useState<{ id: string; username: string } | null>(null)
  const orderedMembers = useMemo(() => {
    const leader = data.squadMembers.find((member) => member.isLeader)
    if (!leader) return data.squadMembers
    return [leader, ...data.squadMembers.filter((member) => member.id !== leader.id)]
  }, [data.squadMembers])

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
      setMemberToKick(null)
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
        {orderedMembers.map((member) => (
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
                onClick={() => setMemberToKick({ id: member.id, username: member.username })}
              >
                {processingMemberId === member.id ? 'Исключение...' : 'Кикнуть'}
              </button>
            ) : null}
          </div>
        ))}
      </div>
      {isLeader && memberToKick && getModalPortalTarget()
        ? createPortal(
            <div className="ui-modal-backdrop" role="presentation" onClick={() => !processingMemberId && setMemberToKick(null)}>
              <div
                className="ui-modal"
                role="dialog"
                aria-modal="true"
                aria-labelledby="kick-member-modal-title"
                onClick={(event) => event.stopPropagation()}
              >
                <div className="ui-modal-header">
                  <h2 id="kick-member-modal-title" className="ui-modal-title">
                    Исключить участника?
                  </h2>
                  <button
                    className="ui-modal-close"
                    type="button"
                    aria-label="Закрыть"
                    onClick={() => setMemberToKick(null)}
                    disabled={Boolean(processingMemberId)}
                  >
                    ×
                  </button>
                </div>
                <div className="ui-modal-body">
                  <p>
                    Участник <strong>{memberToKick.username}</strong> будет исключен из сквада. Подтвердите действие.
                  </p>
                </div>
                <div className="ui-modal-footer">
                  <button className="btn" type="button" onClick={() => setMemberToKick(null)} disabled={Boolean(processingMemberId)}>
                    Отменить
                  </button>
                  <button
                    className="btn danger"
                    type="button"
                    onClick={() => void onKick(memberToKick.id)}
                    disabled={Boolean(processingMemberId)}
                  >
                    {processingMemberId ? 'Исключение...' : 'Кикнуть'}
                  </button>
                </div>
              </div>
            </div>,
            getModalPortalTarget()!,
          )
        : null}
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
  const [isLeaveModalOpen, setIsLeaveModalOpen] = useState(false)

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
      setIsLeaveModalOpen(false)
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
        <button className="btn danger" type="button" disabled={isLeaving} onClick={() => setIsLeaveModalOpen(true)}>
          {isLeaving ? 'Выход...' : 'Покинуть сквад'}
        </button>
      </div>
      {isLeaveModalOpen && getModalPortalTarget()
        ? createPortal(
            <div className="ui-modal-backdrop" role="presentation" onClick={() => !isLeaving && setIsLeaveModalOpen(false)}>
              <div
                className="ui-modal"
                role="dialog"
                aria-modal="true"
                aria-labelledby="leave-squad-modal-title"
                onClick={(event) => event.stopPropagation()}
              >
                <div className="ui-modal-header">
                  <h2 id="leave-squad-modal-title" className="ui-modal-title">
                    Покинуть сквад?
                  </h2>
                  <button
                    className="ui-modal-close"
                    type="button"
                    aria-label="Закрыть"
                    onClick={() => setIsLeaveModalOpen(false)}
                    disabled={isLeaving}
                  >
                    ×
                  </button>
                </div>
                <div className="ui-modal-body">
                  <p>Вы покинете текущий сквад. Подтвердите действие.</p>
                </div>
                <div className="ui-modal-footer">
                  <button className="btn" type="button" onClick={() => setIsLeaveModalOpen(false)} disabled={isLeaving}>
                    Отменить
                  </button>
                  <button className="btn danger" type="button" onClick={() => void onLeave()} disabled={isLeaving}>
                    {isLeaving ? 'Выход...' : 'Покинуть'}
                  </button>
                </div>
              </div>
            </div>,
            getModalPortalTarget()!,
          )
        : null}
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
  const [hasInvalidNameInput, setHasInvalidNameInput] = useState(false)
  const [isCreating, setIsCreating] = useState(false)
  const [processingInviteId, setProcessingInviteId] = useState<string | null>(null)
  const squadNamePattern = data.squadConfig.nameRegex

  const onNameDraftChange = (value: string) => {
    setNameDraft(value)
    setHasInvalidNameInput(hasInvalidSquadNameByConfig(value.trim(), data.squadConfig))
  }

  const onCreate = async () => {
    const trimmed = nameDraft.trim()
    if (hasInvalidSquadNameByConfig(trimmed, data.squadConfig)) {
      setHasInvalidNameInput(true)
      return
    }
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
      setHasInvalidNameInput(false)
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
          <div className={`ui-field ${hasInvalidNameInput ? 'ui-field-error' : ''}`}>
            <label className="ui-label" htmlFor="create-squad-name">Название</label>
            <input
              id="create-squad-name"
              className="ui-input"
              value={nameDraft}
              onChange={(event) => onNameDraftChange(event.target.value)}
              placeholder="Введите название сквада"
              pattern={squadNamePattern}
              minLength={data.squadConfig.nameMinChars}
              maxLength={data.squadConfig.nameMaxChars}
              inputMode="text"
              autoComplete="off"
            />
            <div className={`ui-hint ${hasInvalidNameInput ? 'ui-hint-error' : ''}`}>
              Лимиты: {data.squadConfig.nameMinChars}–{data.squadConfig.nameMaxChars} символов. Разрешены: латиница, кириллица и `-`. Имя не может начинаться или заканчиваться на `-`. Инвайт действует {data.squadConfig.inviteTtlHours} часа.
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
          <div className="profile-title-badge-row">
            <h2 className="card-title">Мои приглашения</h2>
            <span className="ui-badge ui-badge-warning">{data.squadInvites.length}</span>
          </div>
        </div>
        {data.squadInvites.length ? (
          <div className="profile-stack">
            {data.squadInvites.map((invite) => (
              <div key={invite.id} className="profile-invite-card">
                <div className="profile-stack profile-invite-card__content">
                  <strong>{invite.squadName}</strong>
                  <span className="profile-subtle">Истекает {formatDateTime(invite.expiresAt)}</span>
                </div>
                <div className="profile-actions">
                  <button
                    className="btn primary profile-invite-action-btn"
                    type="button"
                    disabled={processingInviteId === invite.id}
                    onClick={() => void onAcceptInvite(invite.id)}
                  >
                    {processingInviteId === invite.id ? 'Обработка...' : 'Принять'}
                  </button>
                  <button
                    className="btn profile-invite-action-btn"
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
            <SquadInfoCard data={displayData} isLeader authToken={authToken} onChanged={onChanged} />
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
          <SquadInfoCard data={displayData} isLeader={false} authToken={authToken} onChanged={onChanged} />
          <MemberActionsCard authToken={authToken} squadId={displayData.squad.id} onChanged={onChanged} />
        </div>
        <SquadMembersCard data={displayData} isLeader={false} authToken={authToken} onChanged={onChanged} />
      </div>
    </div>
  )
}
