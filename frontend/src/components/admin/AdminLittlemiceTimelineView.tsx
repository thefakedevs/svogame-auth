import { useEffect, useMemo, useState } from 'react'
import { getAdminUser, type AdminUserResponse } from '../../api/admin'
import { toDisplayError } from '../../api/http'
import {
  listAdminLittlemiceChecks,
  type LittlemiceCheckListItemResponse,
  type LittlemiceCheckListResponse,
} from '../../api/littlemice'
import { adminUserLittlemiceCheckPath, adminUserLittlemicePath, paths } from '../../routes/paths'
import { pushUrl } from '../../shared/navigation/history'
import ErrorState from '../ErrorState'
import LoadingState from '../LoadingState'

const PAGE_SIZE = 30

const dateTimeFormatter = new Intl.DateTimeFormat('ru-RU', {
  day: '2-digit',
  month: 'short',
  year: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
})

function formatDateTime(value?: string | null) {
  if (!value) return '—'
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : dateTimeFormatter.format(date).replace(', ', ' ')
}

function statusLabel(status: string) {
  if (status === 'pending') return 'Ожидает'
  if (status === 'passed') return 'Пройдена'
  if (status === 'failed_timeout') return 'Таймаут'
  if (status === 'failed_client_error') return 'Ошибка клиента'
  return status
}

function statusClassName(status: string) {
  if (status === 'passed') return 'ui-badge ui-badge-success'
  if (status === 'pending') return 'ui-badge ui-badge-secondary'
  if (status.startsWith('failed')) return 'ui-badge ui-badge-warning'
  return 'ui-badge ui-badge-neutral'
}

function finishTime(item: LittlemiceCheckListItemResponse) {
  return item.completedAt ?? item.receivedAt ?? null
}

function getPaginationPages(page: number, totalPages: number) {
  const pages = new Set([1, totalPages, page - 1, page, page + 1].filter((item) => item >= 1 && item <= totalPages))
  return Array.from(pages).sort((left, right) => left - right)
}

function LittlemicePagination({
  page,
  totalPages,
  onPageChange,
}: {
  page: number
  totalPages: number
  onPageChange: (page: number) => void
}) {
  const pages = getPaginationPages(page, totalPages)

  return (
    <nav className="admin-pagination" aria-label="Страницы проверок littlemice">
      <ul className="ui-pagination">
        <li>
          <button type="button" className="ui-pagination-btn" aria-label="Предыдущая страница" disabled={page <= 1} onClick={() => onPageChange(Math.max(1, page - 1))}>‹</button>
        </li>
        {pages.map((item, index) => (
          <li key={item}>
            {index > 0 && item - pages[index - 1] > 1 ? <span className="ui-pagination-ellipsis">…</span> : null}
            <button
              type="button"
              className="ui-pagination-btn"
              aria-label={`Страница ${item}`}
              aria-current={page === item ? 'page' : undefined}
              onClick={() => onPageChange(item)}
            >
              {item}
            </button>
          </li>
        ))}
        <li>
          <button type="button" className="ui-pagination-btn" aria-label="Следующая страница" disabled={page >= totalPages} onClick={() => onPageChange(Math.min(totalPages, page + 1))}>›</button>
        </li>
      </ul>
    </nav>
  )
}

function playerLabel(user: AdminUserResponse | null | undefined, playerUuid: string) {
  if (user) return user.username
  if (user === null) return playerUuid
  return 'Загрузка...'
}

export default function AdminLittlemiceTimelineView({ token }: { token: string }) {
  const [page, setPage] = useState(1)
  const [response, setResponse] = useState<LittlemiceCheckListResponse | null>(null)
  const [usersById, setUsersById] = useState<Record<string, AdminUserResponse | null>>({})
  const [error, setError] = useState('')

  useEffect(() => {
    let cancelled = false
    const run = async () => {
      setResponse(null)
      setError('')
      try {
        const loaded = await listAdminLittlemiceChecks(token, { page, perPage: PAGE_SIZE })
        if (!cancelled) setResponse(loaded)
      } catch (cause) {
        if (!cancelled) setError(toDisplayError(cause, 'Не удалось загрузить общий таймлайн littlemice.'))
      }
    }
    void run()
    return () => {
      cancelled = true
    }
  }, [page, token])

  useEffect(() => {
    if (!response) return
    const missingIds = Array.from(new Set(response.items.map((item) => item.playerUuid))).filter((id) => !(id in usersById))
    if (!missingIds.length) return

    let cancelled = false
    const run = async () => {
      const loaded = await Promise.all(
        missingIds.map(async (id) => {
          try {
            return [id, await getAdminUser(token, id)] as const
          } catch {
            return [id, null] as const
          }
        }),
      )
      if (!cancelled) {
        setUsersById((current) => ({ ...current, ...Object.fromEntries(loaded) }))
      }
    }
    void run()
    return () => {
      cancelled = true
    }
  }, [response, token, usersById])

  const totalPages = Math.max(1, response?.totalPages ?? 1)
  const loadedUserCount = useMemo(() => {
    if (!response) return 0
    return new Set(response.items.map((item) => item.playerUuid)).size
  }, [response])

  return (
    <div className="admin-page">
      <section className="admin-top-actions">
        <button type="button" className="btn btn-sm" onClick={() => pushUrl(paths.admin)}>
          ← К админке
        </button>
      </section>

      <section className="card admin-card">
        <div className="admin-littlemice-head">
          <div>
            <h2 className="card-title">Все проверки littlemice</h2>
            <p className="card-text">Общий таймлайн проверок по всем игрокам.</p>
          </div>
          {response ? (
            <div className="admin-littlemice-summary-badges">
              <span className="ui-badge ui-badge-neutral">Всего: {response.total}</span>
              <span className="ui-badge ui-badge-neutral">Игроков на странице: {loadedUserCount}</span>
            </div>
          ) : null}
        </div>

        {!response && !error ? <LoadingState title="Загружаем проверки" /> : null}
        {error ? <ErrorState title="Таймлайн недоступен" message={error} /> : null}
        {response ? (
          response.items.length > 0 ? (
            <>
              <div className="admin-littlemice-timeline admin-littlemice-timeline--global">
                {response.items.map((item) => {
                  const user = usersById[item.playerUuid]
                  return (
                    <button
                      key={item.id}
                      type="button"
                      className="admin-littlemice-timeline-item admin-littlemice-global-item"
                      onClick={() => pushUrl(adminUserLittlemiceCheckPath(item.playerUuid, item.id))}
                    >
                      <span className="admin-littlemice-timeline-point" aria-hidden="true" />
                      <span className="admin-littlemice-timeline-main">
                        <span className="admin-littlemice-timeline-head">
                          <span
                            role="button"
                            tabIndex={0}
                            className="admin-littlemice-player-button"
                            onClick={(event) => {
                              event.stopPropagation()
                              pushUrl(adminUserLittlemicePath(item.playerUuid))
                            }}
                            onKeyDown={(event) => {
                              if (event.key !== 'Enter' && event.key !== ' ') return
                              event.preventDefault()
                              event.stopPropagation()
                              pushUrl(adminUserLittlemicePath(item.playerUuid))
                            }}
                          >
                            {playerLabel(user, item.playerUuid)}
                          </span>
                          <span className={statusClassName(item.status)}>{statusLabel(item.status)}</span>
                        </span>
                        <span className="admin-littlemice-timeline-meta">
                          <span>ID: {item.id}</span>
                          <span>Завершение: {formatDateTime(finishTime(item))}</span>
                        </span>
                        <span className="admin-littlemice-global-actions">
                          <code>{item.playerUuid}</code>
                        </span>
                        {item.failureReason ? <span className="admin-littlemice-failure">{item.failureReason}</span> : null}
                      </span>
                    </button>
                  )
                })}
              </div>
              {totalPages > 1 ? <LittlemicePagination page={page} totalPages={totalPages} onPageChange={setPage} /> : null}
            </>
          ) : (
            <p className="admin-inline-muted">Проверок littlemice пока нет.</p>
          )
        ) : null}
      </section>
    </div>
  )
}
