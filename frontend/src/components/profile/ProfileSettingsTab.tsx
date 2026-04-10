import type { Dispatch, SetStateAction } from 'react'
import toast from 'react-hot-toast'
import type { UserProfile } from '../../api/auth'
import { updateNickname } from '../../api/users'
import { toDisplayError } from '../../api/http'
import { buildSkinUrl } from '../../api/skins'
import type { ProfileDashboardData } from './types'
import SkinPreview2D from '../SkinPreview2D'
import SkinUploadInline from '../SkinUploadInline'

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
  return Number.isNaN(date.getTime()) ? value : dateTimeFormatter.format(date)
}

export default function ProfileSettingsTab({
  data,
  authToken,
  nicknameDraft,
  setNicknameDraft,
  isUpdatingNickname,
  setIsUpdatingNickname,
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
  nicknameDraft: string
  setNicknameDraft: (value: string) => void
  isUpdatingNickname: boolean
  setIsUpdatingNickname: (value: boolean) => void
  setData: Dispatch<SetStateAction<ProfileDashboardData | null>>
  setAuthUser: (user: UserProfile | null) => void
  onLogout: () => void
  skinFailed: boolean
  setSkinFailed: (value: boolean) => void
  skinVersion: number
  setSkinVersion: (value: number) => void
}) {
  const onSaveNickname = async () => {
    if (!authToken) return

    const nickname = nicknameDraft.trim()
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
      setAuthUser({ id: user.id, username: user.username, avatarUrl: user.avatarUrl ?? '' })
      setNicknameDraft(user.username)
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
          <div className="ui-field">
            <label className="ui-label" htmlFor="profile-nickname">Никнейм</label>
            <input
              id="profile-nickname"
              className="ui-input"
              value={nicknameDraft}
              onChange={(event) => setNicknameDraft(event.target.value)}
              maxLength={16}
              placeholder="Введите никнейм"
              disabled={isUpdatingNickname}
            />
            <div className="ui-hint">Изменение сохранится в учетной записи после подтверждения.</div>
          </div>
          <div className="profile-actions">
            <button className="btn primary" type="button" disabled={isUpdatingNickname} onClick={() => void onSaveNickname()}>
              {isUpdatingNickname ? 'Сохранение...' : 'Сохранить никнейм'}
            </button>
          </div>
        </div>
        <dl className="profile-kv profile-kv--wide">
          <div><dt>E-mail</dt><dd>{data.user.email ?? 'Не указан'}</dd></div>
          <div><dt>Discord</dt><dd>{data.user.discordId}</dd></div>
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
