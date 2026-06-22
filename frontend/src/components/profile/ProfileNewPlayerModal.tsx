import { useEffect, useMemo, useState } from 'react'
import { listDiscordGuildEvents, type DiscordGuildEventResponse } from '../../api/discord'
import { paths } from '../../routes/paths'
import AppPortal from '../../shared/ui/portal/AppPortal'

type PlatformId = 'windows' | 'linux'

type DownloadOption = {
  id: PlatformId
  title: string
  label: string
  description: string
  image: string
  href: string
}

type EventsState =
  | { status: 'loading' }
  | { status: 'ready'; event: DiscordGuildEventResponse | null }
  | { status: 'error' }

type NavigatorWithUserAgentData = Navigator & {
  userAgentData?: {
    platform?: string
  }
}

const DISCORD_INVITE_URL = 'https://discord.gg/UQQK4ykxMa'

const downloadOptions: Record<PlatformId, DownloadOption> = {
  windows: {
    id: 'windows',
    title: 'Windows',
    label: 'Windows x64',
    description: 'Подходит для Windows 10/11',
    image: '/icons/windows.svg',
    href: 'https://launcher.svocraft.xyz/api/v1/file/win-x64-SvoLauncher.exe',
  },
  linux: {
    id: 'linux',
    title: 'Linux',
    label: 'Linux x64',
    description: 'Универсальная сборка для популярных Linux-дистрибутивов.',
    image: '/icons/linux.svg',
    href: 'https://launcher.svocraft.xyz/api/v1/file/linux-x64-SvoLauncher',
  },
}

const eventDateFormatter = new Intl.DateTimeFormat('ru-RU', {
  day: 'numeric',
  month: 'short',
  hour: '2-digit',
  minute: '2-digit',
})

function detectPlatform(): PlatformId {
  if (typeof navigator === 'undefined') return 'windows'

  const nav = navigator as NavigatorWithUserAgentData
  const platform = `${nav.userAgentData?.platform ?? navigator.platform ?? navigator.userAgent}`.toLowerCase()

  if (platform.includes('linux')) return 'linux'
  return 'windows'
}

function pickNearestEvent(events: DiscordGuildEventResponse[]) {
  const now = Date.now()

  return events
    .filter((event) => {
      if (event.status === 'completed' || event.status === 'canceled') return false
      if (event.status === 'active') return true
      const end = event.endsAt ? new Date(event.endsAt).getTime() : null
      if (event.status === 'scheduled' && end !== null && end < now) return false
      return true
    })
    .sort((left, right) => {
      const priority = (event: DiscordGuildEventResponse) => (event.status === 'active' ? 0 : 1)
      const priorityOrder = priority(left) - priority(right)
      if (priorityOrder !== 0) return priorityOrder

      return new Date(left.startsAt).getTime() - new Date(right.startsAt).getTime()
    })[0] ?? null
}

function eventStatusLabel(event: DiscordGuildEventResponse) {
  if (event.status === 'active') return 'Игра идет сейчас'
  return eventDateFormatter.format(new Date(event.startsAt))
}

function EventPanel({ state }: { state: EventsState }) {
  if (state.status === 'loading') {
    return <p className="profile-onboarding-muted">Загружаем ближайший ивент...</p>
  }

  if (state.status === 'error') {
    return <p className="profile-onboarding-muted">Не удалось загрузить расписание. Актуальные анонсы есть в Discord.</p>
  }

  if (!state.event) {
    return <p className="profile-onboarding-muted">Пока нет запланированных ивентов. Следите за анонсами в Discord.</p>
  }

  return (
    <div className="profile-onboarding-event">
      <span className={state.event.status === 'active' ? 'ui-badge ui-badge-success' : 'ui-badge ui-badge-neutral'}>
        {eventStatusLabel(state.event)}
      </span>
      <strong>{state.event.name}</strong>
      {state.event.location ? <span>{state.event.location}</span> : null}
    </div>
  )
}

export default function ProfileNewPlayerModal({ onClose }: { onClose: () => void }) {
  const [eventsState, setEventsState] = useState<EventsState>({ status: 'loading' })
  const download = useMemo(() => downloadOptions[detectPlatform()], [])

  useEffect(() => {
    let cancelled = false

    listDiscordGuildEvents()
      .then((events) => {
        if (!cancelled) setEventsState({ status: 'ready', event: pickNearestEvent(events) })
      })
      .catch(() => {
        if (!cancelled) setEventsState({ status: 'error' })
      })

    return () => {
      cancelled = true
    }
  }, [])

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose()
    }

    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [onClose])

  return (
    <AppPortal>
      <div className="profile-onboarding-backdrop" role="presentation" onMouseDown={onClose}>
        <section
          className="card profile-onboarding-modal"
          role="dialog"
          aria-modal="true"
          aria-labelledby="profile-onboarding-title"
          onMouseDown={(event) => event.stopPropagation()}
        >
          <div className="ui-card-header">
            <div>
              <span className="ui-badge ui-badge-accent">Новый аккаунт</span>
              <h2 id="profile-onboarding-title" className="card-title">Добро пожаловать</h2>
            </div>
            <button className="profile-onboarding-close" type="button" onClick={onClose} aria-label="Закрыть">
              ×
            </button>
          </div>

          <p className="profile-subtle">
            Аккаунт создан. Осталось скачать лаунчер, зайти в Discord и дождаться ближайшей игры.
          </p>

          <div className="profile-onboarding-grid">
            <article className="profile-onboarding-download">
              <div className="profile-onboarding-download__image">
                <img src={download.image} alt="" loading="lazy" decoding="async" />
              </div>
              <div className="profile-onboarding-download__body">
                <div>
                  <span className="ui-badge ui-badge-success">Ваша ОС</span>
                  <h3>{download.title}</h3>
                </div>
                <p>{download.description}</p>
                <a className="btn primary" href={download.href} download data-analytics-event="download_launcher" data-cta={`profile-onboarding-${download.id}`}>
                  Скачать
                  <span>{download.label}</span>
                </a>
                <a className="profile-onboarding-link" href={paths.downloads} target="_blank" rel="noreferrer">Все версии лаунчера</a>
              </div>
            </article>

            <article className="profile-onboarding-panel profile-onboarding-panel--discord">
              <span className="profile-stat-label">Discord</span>
              <strong>Зайдите на сервер</strong>
              <p>Там анонсы матчей, сборы команд и помощь новичкам.</p>
              <a className="btn btn-secondary-accent" href={DISCORD_INVITE_URL} target="_blank" rel="noreferrer">
                Открыть Discord
              </a>
            </article>

            <article className="profile-onboarding-panel profile-onboarding-panel--event">
              <span className="profile-stat-label">Ближайший ивент</span>
              <EventPanel state={eventsState} />
            </article>
          </div>

          <div className="ui-card-footer">
            <button className="btn" type="button" onClick={onClose}>
              Понятно
            </button>
          </div>
        </section>
      </div>
    </AppPortal>
  )
}
