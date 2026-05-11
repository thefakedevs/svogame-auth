import { useEffect, useMemo, useState } from 'react'
import {
  getAdminLittlemiceBinaryContent,
  getAdminLittlemiceCheck,
  getAdminLittlemiceTextContent,
  listAdminUserLittlemiceChecks,
  type LittlemiceCheckDetailResponse,
  type LittlemiceCheckListItemResponse,
  type LittlemiceCheckListResponse,
} from '../../api/littlemice'
import { toDisplayError } from '../../api/http'
import { adminUserLittlemiceCheckPath, adminUserLittlemicePath, adminUserPath } from '../../routes/paths'
import { pushUrl } from '../../shared/navigation/history'
import ErrorState from '../ErrorState'
import LoadingState from '../LoadingState'

const PAGE_SIZE = 20

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

function formatBytes(value?: number | null) {
  if (value == null) return '—'
  if (value < 1024) return `${value} Б`
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} КБ`
  return `${(value / 1024 / 1024).toFixed(1)} МБ`
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

function LittlemiceCheckSummary({ item, userId }: { item: LittlemiceCheckListItemResponse; userId: string }) {
  return (
    <button
      type="button"
      className="admin-littlemice-timeline-item"
      onClick={() => pushUrl(adminUserLittlemiceCheckPath(userId, item.id))}
    >
      <span className="admin-littlemice-timeline-point" aria-hidden="true" />
      <span className="admin-littlemice-timeline-main">
        <span className="admin-littlemice-timeline-head">
          <strong>{formatDateTime(item.requestedAt)}</strong>
          <span className={statusClassName(item.status)}>{statusLabel(item.status)}</span>
        </span>
        <span className="admin-littlemice-timeline-meta">
          <span>{item.serviceSystemName}</span>
          <span>ID: {item.id}</span>
          <span>Завершение: {formatDateTime(finishTime(item))}</span>
        </span>
        {item.failureReason ? <span className="admin-littlemice-failure">{item.failureReason}</span> : null}
      </span>
    </button>
  )
}

function AdminUserLittlemiceListView({ token, userId }: { token: string; userId: string }) {
  const [page, setPage] = useState(1)
  const [response, setResponse] = useState<LittlemiceCheckListResponse | null>(null)
  const [error, setError] = useState('')

  useEffect(() => {
    let cancelled = false
    const run = async () => {
      setResponse(null)
      setError('')
      try {
        const loaded = await listAdminUserLittlemiceChecks(token, userId, { page, perPage: PAGE_SIZE })
        if (!cancelled) setResponse(loaded)
      } catch (cause) {
        if (!cancelled) setError(toDisplayError(cause, 'Не удалось загрузить проверки littlemice.'))
      }
    }
    void run()
    return () => {
      cancelled = true
    }
  }, [page, token, userId])

  const totalPages = Math.max(1, response?.totalPages ?? 1)

  return (
    <div className="admin-page">
      <section className="admin-top-actions">
        <button type="button" className="btn btn-sm" onClick={() => pushUrl(adminUserPath(userId))}>
          ← К профилю игрока
        </button>
      </section>

      <section className="card admin-card">
        <div className="admin-littlemice-head">
          <div>
            <h2 className="card-title">Проверки littlemice</h2>
            <p className="card-text">User ID: <code>{userId}</code></p>
          </div>
          {response ? <span className="ui-badge ui-badge-neutral">Всего: {response.total}</span> : null}
        </div>

        {!response && !error ? <LoadingState title="Загружаем проверки" /> : null}
        {error ? <ErrorState title="Проверки недоступны" message={error} /> : null}
        {response ? (
          response.items.length > 0 ? (
            <>
              <div className="admin-littlemice-timeline">
                {response.items.map((item) => (
                  <LittlemiceCheckSummary key={item.id} item={item} userId={userId} />
                ))}
              </div>
              {totalPages > 1 ? <LittlemicePagination page={page} totalPages={totalPages} onPageChange={setPage} /> : null}
            </>
          ) : (
            <p className="admin-inline-muted">У игрока еще нет проверок littlemice.</p>
          )
        ) : null}
      </section>
    </div>
  )
}

function useAuthorizedImageUrl(token: string, url: string | null) {
  const [loadedImage, setLoadedImage] = useState<{ sourceUrl: string; objectUrl: string } | null>(null)
  const [loadError, setLoadError] = useState<{ sourceUrl: string; message: string } | null>(null)

  useEffect(() => {
    if (!url) return

    let cancelled = false
    let nextObjectUrl: string | null = null
    const run = async () => {
      try {
        const blob = await getAdminLittlemiceBinaryContent(token, url)
        if (cancelled) return
        nextObjectUrl = URL.createObjectURL(blob)
        setLoadedImage({ sourceUrl: url, objectUrl: nextObjectUrl })
        setLoadError(null)
      } catch (cause) {
        if (!cancelled) setLoadError({ sourceUrl: url, message: toDisplayError(cause, 'Не удалось загрузить изображение.') })
      }
    }
    void run()
    return () => {
      cancelled = true
      if (nextObjectUrl) URL.revokeObjectURL(nextObjectUrl)
    }
  }, [token, url])

  return {
    objectUrl: url && loadedImage?.sourceUrl === url ? loadedImage.objectUrl : null,
    error: url && loadError?.sourceUrl === url ? loadError.message : '',
  }
}

function ScreenshotPanel({
  title,
  url,
  sizeBytes,
  token,
}: {
  title: string
  url: string | null
  sizeBytes: number | null
  token: string
}) {
  const image = useAuthorizedImageUrl(token, url)

  return (
    <section className="admin-littlemice-media-panel">
      <div className="admin-littlemice-media-head">
        <h3>{title}</h3>
        <small>{formatBytes(sizeBytes)}</small>
      </div>
      {!url ? <p className="admin-inline-muted">Файл не загружен.</p> : null}
      {url && !image.objectUrl && !image.error ? <LoadingState title="Загружаем скриншот" /> : null}
      {image.error ? <ErrorState message={image.error} /> : null}
      {image.objectUrl ? <img className="admin-littlemice-screenshot" src={image.objectUrl} alt={title} /> : null}
    </section>
  )
}

function AdminUserLittlemiceDetailView({ token, userId, checkId }: { token: string; userId: string; checkId: string }) {
  const [check, setCheck] = useState<LittlemiceCheckDetailResponse | null>(null)
  const [logText, setLogText] = useState<string | null>(null)
  const [error, setError] = useState('')
  const [logError, setLogError] = useState('')

  useEffect(() => {
    let cancelled = false
    const run = async () => {
      setCheck(null)
      setLogText(null)
      setError('')
      setLogError('')
      try {
        const loaded = await getAdminLittlemiceCheck(token, checkId)
        if (cancelled) return
        setCheck(loaded)

        if (loaded.logUrl) {
          try {
            const text = await getAdminLittlemiceTextContent(token, loaded.logUrl)
            if (!cancelled) setLogText(text)
          } catch (cause) {
            if (!cancelled) setLogError(toDisplayError(cause, 'Не удалось загрузить лог проверки.'))
          }
        }
      } catch (cause) {
        if (!cancelled) setError(toDisplayError(cause, 'Не удалось загрузить проверку littlemice.'))
      }
    }
    void run()
    return () => {
      cancelled = true
    }
  }, [checkId, token])

  const playerMismatch = useMemo(() => check && check.playerUuid !== userId, [check, userId])

  return (
    <div className="admin-page">
      <section className="admin-top-actions">
        <button type="button" className="btn btn-sm" onClick={() => pushUrl(adminUserLittlemicePath(userId))}>
          ← К проверкам littlemice
        </button>
        <button type="button" className="btn btn-sm" onClick={() => pushUrl(adminUserPath(userId))}>
          К профилю игрока
        </button>
      </section>

      <section className="card admin-card">
        {!check && !error ? <LoadingState title="Загружаем проверку" /> : null}
        {error ? <ErrorState title="Проверка недоступна" message={error} /> : null}
        {check ? (
          <div className="admin-littlemice-detail">
            <div className="admin-littlemice-head">
              <div>
                <h2 className="card-title">Проверка littlemice</h2>
                <p className="card-text">Check ID: <code>{check.id}</code></p>
              </div>
              <span className={statusClassName(check.status)}>{statusLabel(check.status)}</span>
            </div>

            {playerMismatch ? (
              <p className="admin-inline-warning">Эта проверка относится к игроку <code>{check.playerUuid}</code>, а открыта из профиля <code>{userId}</code>.</p>
            ) : null}

            <dl className="admin-kv admin-kv--compact admin-littlemice-kv">
              <div><dt>Игрок</dt><dd><code>{check.playerUuid}</code></dd></div>
              <div><dt>Сервис</dt><dd>{check.serviceSystemName}</dd></div>
              <div><dt>Service token</dt><dd><code>{check.serviceTokenId}</code></dd></div>
              <div><dt>Запрошена</dt><dd>{formatDateTime(check.requestedAt)}</dd></div>
              <div><dt>Данные получены</dt><dd>{formatDateTime(check.receivedAt)}</dd></div>
              <div><dt>Завершена</dt><dd>{formatDateTime(check.completedAt)}</dd></div>
              <div><dt>Истекает</dt><dd>{formatDateTime(check.expiresAt)}</dd></div>
              <div><dt>Причина ошибки</dt><dd>{check.failureReason ?? '—'}</dd></div>
            </dl>

            <div className="admin-littlemice-media-grid">
              <ScreenshotPanel title="Скриншот" url={check.screenshotUrl} sizeBytes={check.screenshotSizeBytes} token={token} />
              <ScreenshotPanel title="Скриншот 2" url={check.screenshot2Url} sizeBytes={check.screenshot2SizeBytes} token={token} />
            </div>

            <section className="admin-littlemice-info-grid">
              <article className="admin-littlemice-text-panel">
                <div className="admin-littlemice-media-head">
                  <h3>Client info</h3>
                  <small>{formatBytes(check.clientInfoSizeBytes)}</small>
                </div>
                {check.clientInfoText ? <pre>{check.clientInfoText}</pre> : <p className="admin-inline-muted">Client info не загружен.</p>}
              </article>

              <article className="admin-littlemice-text-panel">
                <div className="admin-littlemice-media-head">
                  <h3>Log</h3>
                  <small>{formatBytes(check.logSizeBytes)}</small>
                </div>
                {!check.logUrl ? <p className="admin-inline-muted">Лог не загружен.</p> : null}
                {check.logUrl && logText == null && !logError ? <LoadingState title="Загружаем лог" /> : null}
                {logError ? <ErrorState message={logError} /> : null}
                {logText != null ? <pre>{logText}</pre> : null}
              </article>
            </section>
          </div>
        ) : null}
      </section>
    </div>
  )
}

export default function AdminUserLittlemiceChecksView({
  token,
  userId,
  checkId,
}: {
  token: string
  userId: string
  checkId?: string
}) {
  if (checkId) {
    return <AdminUserLittlemiceDetailView token={token} userId={userId} checkId={checkId} />
  }

  return <AdminUserLittlemiceListView token={token} userId={userId} />
}
