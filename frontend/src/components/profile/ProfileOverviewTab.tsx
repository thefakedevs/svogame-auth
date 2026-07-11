import { useEffect, useState } from 'react'
import { ApiError, toDisplayError } from '../../api/http'
import { getPlayerStats, type PlayerStatsResponse } from '../../api/metrics'
import { buildSkinUrl } from '../../api/skins'
import SkinPreview2D from '../SkinPreview2D'
import ProfileMatchSummary from './ProfileMatchSummary'
import ProfileMatchSummarySkeleton from './ProfileMatchSummarySkeleton'
import type { ProfileDashboardData, ProfileTab } from './types'

const dateTimeFormatter = new Intl.DateTimeFormat('ru-RU', {
  day: '2-digit',
  month: 'short',
  year: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
})

function initials(value: string) {
  return value.slice(0, 2).toUpperCase()
}

function formatDateTime(value?: string | null) {
  if (!value) return 'Нет данных'
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : dateTimeFormatter.format(date).replace(', ', ' ')
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
  setActiveTab: (tab: ProfileTab) => void
  skinVersion: number
}) {
  const [matchStats, setMatchStats] = useState<
    | { status: 'loading' }
    | { status: 'ready'; stats: PlayerStatsResponse }
    | { status: 'empty' }
    | { status: 'error'; message: string }
  >({ status: 'loading' })

  useEffect(() => {
    let cancelled = false
    setMatchStats({ status: 'loading' })
    getPlayerStats(data.user.id)
      .then((stats) => {
        if (!cancelled) setMatchStats({ status: 'ready', stats })
      })
      .catch((error) => {
        if (cancelled) return
        if (error instanceof ApiError && error.status === 404) {
          setMatchStats({ status: 'empty' })
          return
        }
        setMatchStats({ status: 'error', message: toDisplayError(error, 'Не удалось обновить игровую статистику.') })
      })
    return () => { cancelled = true }
  }, [data.user.id])

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
            <dd>{formatDateTime(data.user.lastLoginAt).replace(', ', ' ')}</dd>
          </div>
        </dl>
      </aside>

      <div className="profile-main">
        <section className="card profile-panel">
          <div className="ui-card-header">
            <div>
              <h2 className="card-title">Игровая статистика</h2>
              <p className="profile-subtle">Ваши результаты за всё время.</p>
            </div>
            <button className="btn btn-sm" type="button" onClick={() => setActiveTab('matches')}>
              Все матчи
            </button>
          </div>
          {matchStats.status === 'loading' ? <ProfileMatchSummarySkeleton /> : null}
          {matchStats.status === 'ready' ? <ProfileMatchSummary stats={matchStats.stats} /> : null}
          {matchStats.status === 'empty' ? <div className="profile-empty profile-overview-match-empty"><p>Сыграйте первый матч — здесь появятся ваши результаты.</p></div> : null}
          {matchStats.status === 'error' ? <div className="profile-empty profile-overview-match-empty"><p>{matchStats.message}</p></div> : null}
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
              <div className="profile-squad-head-row profile-squad-head-row--overview">
                {data.squad.imageUrl ? (
                  <img
                    className="profile-squad-avatar-image profile-squad-avatar-image--overview"
                    src={`${data.squad.imageUrl}?v=${encodeURIComponent(data.squad.updatedAt)}`}
                    alt={`Аватар сквада ${data.squad.name}`}
                  />
                ) : null}
                <strong className="profile-squad-name-inline">{data.squad.name}</strong>
              </div>
              <span className="profile-subtle">
                Подробная информация, состав и приглашения доступны во вкладке сквада.
              </span>
            </div>
          ) : (
            <div className="profile-empty">
              <div className="profile-chip-row">
                <span className="ui-badge ui-badge-neutral">Нет сквада</span>
                {data.squadInvites.length > 0 ? (
                  <span className="ui-badge ui-badge-success">Инвайты: {data.squadInvites.length}</span>
                ) : null}
              </div>
              <p>Во вкладке сквадов можно создать команду, принять приглашение или посмотреть доступные действия.</p>
            </div>
          )}
        </section>
      </div>
    </div>
  )
}
