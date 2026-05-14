import { useCallback, useEffect, useMemo, useState } from 'react'
import toast from 'react-hot-toast'
import { toDisplayError } from '../../api/http'
import { listAdminLootboxes, type LootboxDefinitionResponse } from '../../api/lootboxes'
import { adminLootboxPath } from '../../routes/paths'
import { pushUrl } from '../../shared/navigation/history'
import ErrorState from '../ErrorState'
import LoadingState from '../LoadingState'
import AdminLink from './AdminLink'
import AdminLootboxWizard from './AdminLootboxWizard'

type State =
  | { status: 'loading' }
  | { status: 'error'; error: string }
  | { status: 'ready'; items: LootboxDefinitionResponse[] }

type SortKey = 'updated_desc' | 'name_asc' | 'key_asc'
const LOOTBOXES_PER_PAGE = 20

function sortLootboxes(items: LootboxDefinitionResponse[], sort: SortKey) {
  return [...items].sort((a, b) => {
    if (sort === 'name_asc') return a.assetDisplayName.localeCompare(b.assetDisplayName, 'ru')
    if (sort === 'key_asc') return a.assetKey.localeCompare(b.assetKey)
    return b.updatedAt.localeCompare(a.updatedAt)
  })
}

function getPaginationPages(page: number, totalPages: number) {
  const pages = new Set([1, totalPages, page - 1, page, page + 1].filter((item) => item >= 1 && item <= totalPages))
  return Array.from(pages).sort((left, right) => left - right)
}

function AdminPagination({
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
    <nav className="admin-pagination" aria-label="Нумерация страниц лутбоксов">
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

export default function AdminLootboxesPanel({ token }: { token: string }) {
  const [activeTab, setActiveTab] = useState<'manage' | 'wizard'>('manage')
  const [state, setState] = useState<State>({ status: 'loading' })
  const [query, setQuery] = useState('')
  const [sort, setSort] = useState<SortKey>('updated_desc')
  const [page, setPage] = useState(1)

  const load = useCallback(async () => {
    setState({ status: 'loading' })
    try {
      setState({ status: 'ready', items: await listAdminLootboxes(token) })
    } catch (cause) {
      setState({ status: 'error', error: toDisplayError(cause, 'Не удалось загрузить лутбоксы.') })
    }
  }, [token])

  useEffect(() => {
    if (activeTab === 'manage') queueMicrotask(() => void load())
  }, [activeTab, load])

  const visibleItems = useMemo(() => {
    if (state.status !== 'ready') return []
    const needle = query.trim().toLowerCase()
    const filtered = needle
      ? state.items.filter((item) =>
          `${item.assetKey} ${item.assetDisplayName} ${item.assetDescription ?? ''}`.toLowerCase().includes(needle),
        )
      : state.items
    return sortLootboxes(filtered, sort)
  }, [query, sort, state])

  const totalPages = Math.max(1, Math.ceil(visibleItems.length / LOOTBOXES_PER_PAGE))
  const pagedItems = visibleItems.slice((page - 1) * LOOTBOXES_PER_PAGE, page * LOOTBOXES_PER_PAGE)

  useEffect(() => {
    queueMicrotask(() => setPage((current) => Math.min(current, totalPages)))
  }, [totalPages])

  const setQueryAndReset = (value: string) => {
    setQuery(value)
    setPage(1)
  }
  const activeLootboxCount = state.status === 'ready' ? state.items.filter((item) => item.isActive).length : 0
  const hiddenLootboxCount = state.status === 'ready' ? state.items.filter((item) => !item.isPublic).length : 0

  return (
    <div className="admin-lootbox-page">
      <section className="card admin-card admin-lootbox-tabs-card">
        <div className="admin-tabs">
          <button
            type="button"
            className={`admin-tab ${activeTab === 'manage' ? 'is-active' : ''}`}
            onClick={() => setActiveTab('manage')}
          >
            Управление
          </button>
          <button
            type="button"
            className={`admin-tab ${activeTab === 'wizard' ? 'is-active' : ''}`}
            onClick={() => setActiveTab('wizard')}
          >
            Полный цикл
          </button>
        </div>
      </section>

      {activeTab === 'manage' ? (
        <section className="card admin-card admin-lootbox-list">
          <div className="admin-section-head">
            <div>
              <h2 className="card-title">Лутбоксы</h2>
              <p className="card-text">Definition-ы, видимость, активность и содержимое наград.</p>
            </div>
            <button type="button" className="btn btn-sm" onClick={() => void load()}>
              Обновить
            </button>
          </div>

          {state.status === 'ready' ? (
            <div className="admin-metric-strip">
              <div className="admin-metric">
                <span>Всего</span>
                <strong>{state.items.length}</strong>
              </div>
              <div className="admin-metric admin-metric--success">
                <span>Активны</span>
                <strong>{activeLootboxCount}</strong>
              </div>
              <div className="admin-metric admin-metric--warning">
                <span>Скрыты</span>
                <strong>{hiddenLootboxCount}</strong>
              </div>
            </div>
          ) : null}

          <div className="admin-shop-list-controls admin-filter-bar">
            <div className="admin-shop-list-controls-row">
              <label className="admin-shop-field admin-filter-label">
                <span>Поиск</span>
                <input
                  className="ui-input"
                  value={query}
                  onChange={(event) => setQueryAndReset(event.target.value)}
                  placeholder="key, название, описание"
                />
              </label>
              <label className="admin-shop-field admin-filter-label">
                <span>Сортировка</span>
                <select className="ui-input" value={sort} onChange={(event) => setSort(event.target.value as SortKey)}>
                  <option value="updated_desc">Сначала обновленные</option>
                  <option value="name_asc">Название</option>
                  <option value="key_asc">Key</option>
                </select>
              </label>
            </div>
          </div>

          {state.status === 'loading' ? <LoadingState title="Загружаем лутбоксы" /> : null}
          {state.status === 'error' ? (
            <ErrorState title="Лутбоксы недоступны" message={state.error} primaryActionLabel="Повторить" onPrimaryAction={() => void load()} />
          ) : null}
          {state.status === 'ready' ? (
            <>
              <p className="admin-inline-muted">Показано: {visibleItems.length} из {state.items.length}. Страница {page} из {totalPages}</p>
              <div className="admin-list">
                {pagedItems.map((item) => (
                  <AdminLink
                    key={item.id}
                    className="admin-row"
                    href={adminLootboxPath(item.id)}
                  >
                    <span className="admin-row-user-text">
                      <strong>{item.assetDisplayName}</strong>
                      <small>{item.assetKey} · {item.id}</small>
                    </span>
                    <span className="admin-row-badges">
                      <span className={`ui-badge ${item.isActive ? 'ui-badge-success' : 'ui-badge-warning'}`}>
                        {item.isActive ? 'active' : 'inactive'}
                      </span>
                      <span className={`ui-badge ${item.isPublic ? 'ui-badge-secondary' : 'ui-badge-neutral'}`}>
                        {item.isPublic ? 'public' : 'hidden'}
                      </span>
                    </span>
                  </AdminLink>
                ))}
                {!visibleItems.length ? <p className="admin-inline-muted">Лутбоксы не найдены.</p> : null}
              </div>
              {visibleItems.length ? <AdminPagination page={page} totalPages={totalPages} onPageChange={setPage} /> : null}
            </>
          ) : null}
        </section>
      ) : (
        <AdminLootboxWizard token={token} onCreated={(lootboxId) => {
          toast.success('Лутбокс создан.')
          pushUrl(adminLootboxPath(lootboxId))
        }} />
      )}
    </div>
  )
}
