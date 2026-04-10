import { useState } from 'react'
import toast from 'react-hot-toast'
import { acceptSquadInvite, createSquad, declineSquadInvite } from '../../../api/squads'
import { toDisplayError } from '../../../api/http'
import { formatDateTime } from '../lib/format'
import {
  hasInvalidSquadNameBoundary,
  hasInvalidSquadNameByConfig,
  isInvalidSquadNameLength,
  sanitizeSquadName,
} from '../lib/validation'
import type { SquadSectionProps } from '../types'

export default function NoSquadState({ authToken, data, onChanged }: SquadSectionProps) {
  const [nameDraft, setNameDraft] = useState('')
  const [hasInvalidNameInput, setHasInvalidNameInput] = useState(false)
  const [isCreating, setIsCreating] = useState(false)
  const [processingInviteId, setProcessingInviteId] = useState<string | null>(null)

  const onNameDraftChange = (value: string) => {
    const sanitized = sanitizeSquadName(value)
    const trimmed = sanitized.trim()

    setNameDraft(sanitized)
    setHasInvalidNameInput(
      sanitized !== value ||
        hasInvalidSquadNameBoundary(trimmed) ||
        isInvalidSquadNameLength(trimmed) ||
        hasInvalidSquadNameByConfig(trimmed, data.squadConfig),
    )
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
              minLength={data.squadConfig.nameMinChars}
              maxLength={data.squadConfig.nameMaxChars}
              inputMode="text"
              autoComplete="off"
            />
            <div className={`ui-hint ${hasInvalidNameInput ? 'ui-hint-error' : ''}`}>
              Лимиты: {data.squadConfig.nameMinChars}-{data.squadConfig.nameMaxChars} символов. Разрешены: латиница, кириллица и `-`. Имя не может начинаться или заканчиваться на `-`. Инвайт действует {data.squadConfig.inviteTtlHours} часа.
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
                  <button className="btn primary profile-invite-action-btn" type="button" disabled={processingInviteId === invite.id} onClick={() => void onAcceptInvite(invite.id)}>
                    {processingInviteId === invite.id ? 'Обработка...' : 'Принять'}
                  </button>
                  <button className="btn profile-invite-action-btn" type="button" disabled={processingInviteId === invite.id} onClick={() => void onDeclineInvite(invite.id)}>
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
