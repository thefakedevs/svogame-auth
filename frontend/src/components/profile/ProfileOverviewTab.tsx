import { buildSkinUrl } from '../../api/skins'
import SkinPreview2D from '../SkinPreview2D'
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

export default function ProfileOverviewTab({
  data,
  skinFailed,
  setSkinFailed,
  setActiveTab,
  skinVersion,
}: {
  data: ProfileDashboardData
  skinFailed: boolean
  setSkinFailed: (value: boolean) => void
  setActiveTab: (tab: 'overview' | 'squads' | 'settings') => void
  skinVersion: number
}) {
  return (
    <div className="profile-layout">
      <aside className="card profile-identity">
        <div className="profile-identity-head">
          {data.user.avatarUrl ? (
            <img className="profile-avatar" src={data.user.avatarUrl} alt={data.user.username} />
          ) : (
            <div className="ui-avatar profile-avatar-fallback">{initials(data.user.username)}</div>
          )}
          <div>
            <h1 className="profile-name">{data.user.username}</h1>
            <div className="profile-chip-row">
              {data.user.isSuperuser ? <span className="ui-badge ui-badge-secondary">Админ</span> : null}
              <span className="ui-badge ui-badge-neutral">{data.user.isActive ? 'Активен' : 'Отключен'}</span>
            </div>
          </div>
        </div>

        <div className="profile-skin-card">
          <div className="profile-section-head">
            <span className="ui-section-title">Текущий скин</span>
            <button className="btn btn-sm" type="button" onClick={() => setActiveTab('settings')}>
              Изменить
            </button>
          </div>
          <div className="profile-skin-preview">
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
                <p>Загрузите PNG в настройках профиля.</p>
              </div>
            )}
          </div>
        </div>

        <dl className="profile-kv">
          {data.squad ? (
            <div>
              <dt>Состоит в скваде</dt>
              <dd>{data.squad.name}</dd>
            </div>
          ) : null}
          <div>
            <dt>Последний вход</dt>
            <dd>{formatDateTime(data.user.lastLoginAt)}</dd>
          </div>
          <div>
            <dt>Создан</dt>
            <dd>{formatDate(data.user.createdAt)}</dd>
          </div>
          <div>
            <dt>Discord ID</dt>
            <dd>{data.user.discordId}</dd>
          </div>
        </dl>
      </aside>

      <div className="profile-main">
        <section className="card profile-panel">
          <div className="ui-card-header">
            <h2 className="card-title">Состояние аккаунта</h2>
          </div>
          <div className="profile-stack">
            <div className="profile-inline-card">
              <strong>Статус профиля</strong>
              <span className="profile-subtle">
                {data.user.isActive
                  ? 'Аккаунт активен и готов к использованию'
                  : 'Аккаунт временно недоступен'}
              </span>
            </div>
            <div className="profile-inline-card">
              <strong>Никнейм</strong>
              <span className="profile-subtle">{data.user.username}</span>
            </div>
            <div className="profile-inline-card">
              <strong>Скин</strong>
              <span className="profile-subtle">
                {skinFailed ? 'Скин не загружен' : 'Скин загружен и отображается'}
              </span>
            </div>
          </div>
        </section>

        <section className="card profile-panel">
          <div className="ui-card-header">
            <h2 className="card-title">Сквад</h2>
            <button className="btn btn-sm" type="button" onClick={() => setActiveTab('squads')}>
              {data.squad ? 'Открыть страницу сквада' : 'Перейти к сквадам'}
            </button>
          </div>
          {data.squad ? (
            <div className="profile-stack">
              <strong>{data.squad.name}</strong>
              <span className="profile-subtle">
                Подробная информация, состав и приглашения доступны во вкладке сквада.
              </span>
            </div>
          ) : (
            <div className="profile-empty">
              <span className="ui-badge ui-badge-neutral">Нет сквада</span>
              <p>Во вкладке сквадов можно создать команду, принять приглашение или посмотреть доступные действия.</p>
            </div>
          )}
        </section>
      </div>
    </div>
  )
}
