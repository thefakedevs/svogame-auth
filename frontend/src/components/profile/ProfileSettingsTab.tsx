import { useState, type Dispatch, type SetStateAction } from 'react'
import toast from 'react-hot-toast'
import type { UserProfile } from '../../api/auth'
import { buildSkinUrl } from '../../api/skins'
import { toDisplayError } from '../../api/http'
import { updateNickname } from '../../api/users'
import SkinPreview2D from '../SkinPreview2D'
import SkinUploadInline from '../SkinUploadInline'
import type { ProfileDashboardData } from './types'

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

function formatDate(value?: string | null) {
  if (!value) return 'Нет данных'
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : dateFormatter.format(date).replace(', ', ' ')
}

function formatDateTime(value?: string | null) {
  if (!value) return 'Нет данных'
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : dateTimeFormatter.format(date).replace(', ', ' ')
}

function sanitizeNickname(value: string) {
  return value.replace(/[^A-Za-z0-9_]/g, '')
}

function isInvalidNicknameLength(value: string) {
  return value.length < 2 || value.length > 16
}

export default function ProfileSettingsTab({
  data,
  authToken,
  setData,
  setAuthUser,
  onLogout,
  skinFailed,
  setSkinFailed,
  skinVersion,
  setSkinVersion,
}: {
  data: ProfileDashboardData
  authToken: string | null
  setData: Dispatch<SetStateAction<ProfileDashboardData | null>>
  setAuthUser: (user: UserProfile | null) => void
  onLogout: () => void
  skinFailed: boolean
  setSkinFailed: (value: boolean) => void
  skinVersion: number
  setSkinVersion: (value: number) => void
}) {
  const [nicknameDraft, setNicknameDraft] = useState(data.user.username)
  const [isUpdatingNickname, setIsUpdatingNickname] = useState(false)
  const [hasInvalidNicknameInput, setHasInvalidNicknameInput] = useState(false)

  const onNicknameChange = (value: string) => {
    const sanitizedValue = sanitizeNickname(value)
    setHasInvalidNicknameInput(sanitizedValue !== value || isInvalidNicknameLength(sanitizedValue.trim()))
    setNicknameDraft(sanitizedValue)
  }

  const onSaveNickname = async () => {
    if (!authToken) return

    const nickname = nicknameDraft.trim()
    if (isInvalidNicknameLength(nickname)) {
      setHasInvalidNicknameInput(true)
      return
    }

    if (!nickname || nickname === data.user.username) return

    setIsUpdatingNickname(true)
    const request = updateNickname(authToken, nickname)

    toast.promise(request, {
      loading: 'Сохраняем никнейм...',
      success: 'Никнейм обновлен.',
      error: (error) => toDisplayError(error, 'Не удалось обновить никнейм.'),
    })

    try {
      const user = await request
      setAuthUser({
        id: user.id,
        username: user.username,
        avatarUrl: user.avatarUrl ?? '',
        isSuperuser: user.isSuperuser,
      })
      setNicknameDraft(user.username)
      setHasInvalidNicknameInput(false)
      setData((current) => (current ? { ...current, user } : current))
    } finally {
      setIsUpdatingNickname(false)
    }
  }

  return (
    <div className="profile-split">
      <section className="card profile-panel">
        <div className="ui-card-header"><h2 className="card-title">Данные аккаунта</h2></div>
        <div className="profile-form">
          <div className={`ui-field ${hasInvalidNicknameInput ? 'ui-field-error' : ''}`}>
            <label className="ui-label" htmlFor="profile-nickname">Никнейм</label>
            <input
              id="profile-nickname"
              className="ui-input"
              value={nicknameDraft}
              onChange={(event) => onNicknameChange(event.target.value)}
              minLength={2}
              maxLength={16}
              inputMode="text"
              autoComplete="off"
              placeholder="Введите никнейм"
              disabled={isUpdatingNickname}
            />
            <div className={`ui-hint ${hasInvalidNicknameInput ? 'ui-hint-error' : ''}`}>
              Никнейм: 2-16 символов. Разрешены только латинские буквы, цифры и `_`.
            </div>
          </div>
          <div className="profile-actions">
            <button className="btn primary" type="button" disabled={isUpdatingNickname || hasInvalidNicknameInput} onClick={() => void onSaveNickname()}>
              {isUpdatingNickname ? 'Сохранение...' : 'Сохранить никнейм'}
            </button>
          </div>
        </div>
        <dl className="profile-kv profile-kv--wide">
          <div><dt>E-mail</dt><dd>{data.user.email ?? 'Не указан'}</dd></div>
          <div><dt>Discord</dt><dd>{data.user.discordId}</dd></div>
          <div><dt>Дата регистрации</dt><dd>{formatDate(data.user.createdAt)}</dd></div>
          <div><dt>Последний вход</dt><dd>{formatDateTime(data.user.lastLoginAt)}</dd></div>
        </dl>
      </section>

      <section className="card profile-panel">
        <div className="ui-card-header"><h2 className="card-title">Скин и сессия</h2></div>
        <div className="profile-stack">
          <div className="profile-skin-preview profile-skin-preview--large">
            {!skinFailed ? (
              <SkinPreview2D
                className="profile-skin-render"
                src={buildSkinUrl(data.user.id, skinVersion)}
                alt={`Скин ${data.user.username}`}
                onError={() => setSkinFailed(true)}
              />
            ) : (
              <div className="profile-empty profile-empty--centered">
                <span className="ui-badge ui-badge-neutral">Нет скина</span>
                <p>Загрузите PNG-файл, чтобы добавить внешний вид персонажа.</p>
              </div>
            )}
          </div>
          <SkinUploadInline onUploaded={() => {
            setSkinFailed(false)
            setSkinVersion(Date.now())
          }} />
          <button className="btn danger" type="button" onClick={onLogout}>Выйти из аккаунта</button>
        </div>
      </section>
    </div>
  )
}
