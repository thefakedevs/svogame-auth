import { useCallback, useEffect, useMemo, useState } from 'react'
import {
  listAdminShopOrders,
  listAdminUsers,
  type AdminShopOrderResponse,
} from '../../api/admin'
import { toDisplayError } from '../../api/http'
import { getAdminLootboxOpenHistory, type LootboxOpenHistoryResponse } from '../../api/lootboxes'
import { adminUserPath } from '../../routes/paths'
import ErrorState from '../ErrorState'
import LoadingState from '../LoadingState'
import AdminLink from './AdminLink'

const SHOP_ORDERS_PER_PAGE = 30
const LOOTBOX_OPENINGS_PER_PAGE = 30

const dateTimeFormatter = new Intl.DateTimeFormat('ru-RU', {
  day: '2-digit',
  month: '2-digit',
  year: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
})

function formatDateTime(value?: string | null) {
  if (!value) return 'нет данных'
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : dateTimeFormatter.format(date).replace(',', '')
}

function formatRub(value: number) {
  return `${value.toLocaleString('ru-RU')} ₽`
}

function formatMetadata(value: unknown) {
  if (value == null) return '—'
  try {
    return JSON.stringify(value, null, 2)
  } catch {
    return String(value)
  }
}

function formatLootboxReward(item: LootboxOpenHistoryResponse) {
  const reward = item.reward
  if (reward.amount != null) return `${reward.displayName} x${reward.amount}`
  if (reward.durationSeconds != null) return `${reward.displayName} на ${reward.durationSeconds} сек.`
  return reward.displayName
}

function getPaginationPages(page: number, totalPages: number) {
  const pages = new Set([1, totalPages, page - 1, page, page + 1].filter((item) => item >= 1 && item <= totalPages))
  return Array.from(pages).sort((left, right) => left - right)
}

function AdminPagination({
  page,
  totalPages,
  onPageChange,
  label,
}: {
  page: number
  totalPages: number
  onPageChange: (page: number) => void
  label: string
}) {
  const pages = getPaginationPages(page, totalPages)

  return (
    <nav className="admin-pagination" aria-label={label}>
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

export default function AdminTimelinePanel({ token }: { token: string }) {
  const [orders, setOrders] = useState<AdminShopOrderResponse[] | null>(null)
  const [ordersError, setOrdersError] = useState('')
  const [ordersPage, setOrdersPage] = useState(1)
  const [ordersTotalPages, setOrdersTotalPages] = useState(1)
  const [ordersTotal, setOrdersTotal] = useState(0)
  const [openedOrderId, setOpenedOrderId] = useState<string | null>(null)

  const [lootboxHistory, setLootboxHistory] = useState<LootboxOpenHistoryResponse[] | null>(null)
  const [lootboxError, setLootboxError] = useState('')
  const [lootboxUsers, setLootboxUsers] = useState<Record<string, string>>({})
  const [lootboxPage, setLootboxPage] = useState(1)
  const [openedLootboxId, setOpenedLootboxId] = useState<number | null>(null)

  const loadOrders = useCallback(
    async (page: number) => {
      setOrdersError('')
      setOrders(null)
      try {
        const response = await listAdminShopOrders(token, { page, perPage: SHOP_ORDERS_PER_PAGE })
        setOrders(response.items)
        setOrdersPage(response.page)
        setOrdersTotal(response.total)
        setOrdersTotalPages(Math.max(1, response.totalPages))
      } catch (cause) {
        setOrdersError(toDisplayError(cause, 'Не удалось загрузить покупки магазина.'))
      }
    },
    [token],
  )

  const loadLootboxes = useCallback(async () => {
    setLootboxError('')
    setLootboxHistory(null)
    try {
      const history = await getAdminLootboxOpenHistory(token)
      setLootboxHistory(history)
      setLootboxPage(1)

      const userIds = Array.from(new Set(history.map((item) => item.userId)))
      const users = await Promise.all(
        userIds.map(async (userId) => {
          try {
            const response = await listAdminUsers(token, { q: userId, page: 1, perPage: 1 })
            return [userId, response.items[0]?.username ?? userId] as const
          } catch {
            return [userId, userId] as const
          }
        }),
      )
      setLootboxUsers(Object.fromEntries(users))
    } catch (cause) {
      setLootboxError(toDisplayError(cause, 'Не удалось загрузить открытия кейсов.'))
    }
  }, [token])

  useEffect(() => {
    void loadOrders(ordersPage)
  }, [loadOrders, ordersPage])

  useEffect(() => {
    void loadLootboxes()
  }, [loadLootboxes])

  const lootboxCountLabel = useMemo(() => {
    if (!lootboxHistory) return 'Загружаем открытия кейсов...'
    return `Всего открытий: ${lootboxHistory.length}. Страница ${lootboxPage} из ${Math.max(1, Math.ceil(lootboxHistory.length / LOOTBOX_OPENINGS_PER_PAGE))}`
  }, [lootboxHistory, lootboxPage])

  const lootboxTotalPages = Math.max(1, Math.ceil((lootboxHistory?.length ?? 0) / LOOTBOX_OPENINGS_PER_PAGE))
  const pagedLootboxHistory = (lootboxHistory ?? []).slice(
    (lootboxPage - 1) * LOOTBOX_OPENINGS_PER_PAGE,
    lootboxPage * LOOTBOX_OPENINGS_PER_PAGE,
  )

  useEffect(() => {
    setLootboxPage((current) => Math.min(current, lootboxTotalPages))
  }, [lootboxTotalPages])

  return (
    <div className="admin-shop-layout">
      <section className="card admin-card admin-shop-orders">
        <div className="admin-section-head">
          <div>
            <h2 className="card-title">Покупки</h2>
            <p className="card-text">Кто, что и когда купил.</p>
          </div>
          <button type="button" className="btn btn-sm" onClick={() => void loadOrders(ordersPage)}>
            Обновить
          </button>
        </div>

        <p className="admin-inline-muted admin-zero-margin">
          {orders ? `Всего покупок: ${ordersTotal}. Страница ${ordersPage} из ${ordersTotalPages}` : 'Загружаем покупки...'}
        </p>

        {ordersError ? <ErrorState message={ordersError} /> : null}
        {!orders && !ordersError ? <LoadingState title="Загружаем покупки" /> : null}
        {orders && orders.length === 0 ? <p className="admin-inline-muted">Покупок пока нет.</p> : null}
        {orders && orders.length > 0 ? (
          <div className="admin-purchase-timeline">
            {orders.map(({ order, user }) => {
              const isExpanded = openedOrderId === order.id
              return (
                <article key={order.id} className={`admin-purchase-card${isExpanded ? ' is-expanded' : ''}`}>
                  <button
                    type="button"
                    className="admin-purchase-card-main"
                    aria-expanded={isExpanded}
                    onClick={() => setOpenedOrderId(isExpanded ? null : order.id)}
                  >
                    <span className="admin-purchase-time">{formatDateTime(order.createdAt)}</span>
                    <span className="admin-purchase-summary">
                      <strong>{user.username}</strong>
                      <span>{formatRub(order.totalPriceRub)}</span>
                      <span>{order.productName}</span>
                    </span>
                    <span className={`ui-badge ${order.status === 'fulfilled' ? 'ui-badge-success' : order.status === 'fulfillment_failed' ? 'ui-badge-warning' : 'ui-badge-neutral'}`}>
                      {order.status}
                    </span>
                  </button>

                  {isExpanded ? (
                    <div className="admin-purchase-details">
                      <dl className="admin-kv admin-purchase-kv">
                        <div><dt>Игрок</dt><dd><AdminLink href={adminUserPath(order.userId)}>{user.username}</AdminLink></dd></div>
                        <div><dt>Товар</dt><dd>{order.productName} ({order.productKey})</dd></div>
                        <div><dt>Asset</dt><dd>{order.assetKey} · {order.ownershipModel}</dd></div>
                        <div><dt>Количество</dt><dd>{order.quantity}</dd></div>
                        <div><dt>Сумма</dt><dd>{formatRub(order.totalPriceRub)}</dd></div>
                        <div><dt>Оплата</dt><dd>{order.paymentProvider}</dd></div>
                        <div><dt>Оплачен</dt><dd>{formatDateTime(order.paidAt)}</dd></div>
                        <div><dt>Выдан</dt><dd>{formatDateTime(order.fulfilledAt)}</dd></div>
                      </dl>

                      {order.failureProblem ? <p className="admin-inline-muted">Ошибка: {order.failureProblem}</p> : null}

                      <div className="admin-purchase-detail-grid">
                        <section>
                          <h3>Платеж</h3>
                          {order.payment ? (
                            <dl className="admin-kv admin-purchase-kv">
                              <div><dt>ID</dt><dd>{order.payment.id}</dd></div>
                              <div><dt>Provider ID</dt><dd>{order.payment.providerPaymentId || '—'}</dd></div>
                              <div><dt>Статус</dt><dd>{order.payment.status}</dd></div>
                              <div><dt>Создан</dt><dd>{formatDateTime(order.payment.createdAt)}</dd></div>
                            </dl>
                          ) : <p className="admin-inline-muted">Платежа нет.</p>}
                        </section>

                        <section>
                          <h3>Чек</h3>
                          {order.receipt ? (
                            <dl className="admin-kv admin-purchase-kv">
                              <div><dt>ID</dt><dd>{order.receipt.id}</dd></div>
                              <div><dt>Статус</dt><dd>{order.receipt.displayStatus}</dd></div>
                              <div><dt>UUID</dt><dd>{order.receipt.receiptUuid ?? '—'}</dd></div>
                              <div><dt>Попытки</dt><dd>{order.receipt.attemptCount}</dd></div>
                            </dl>
                          ) : <p className="admin-inline-muted">Чека нет.</p>}
                        </section>
                      </div>
                    </div>
                  ) : null}
                </article>
              )
            })}
          </div>
        ) : null}
        {orders && orders.length > 0 ? (
          <AdminPagination page={ordersPage} totalPages={ordersTotalPages} onPageChange={setOrdersPage} label="Нумерация страниц покупок" />
        ) : null}
      </section>

      <section className="card admin-card admin-shop-orders">
        <div className="admin-section-head">
          <div>
            <h2 className="card-title">Открытия кейсов</h2>
            <p className="card-text">Кто открывал кейсы и что выпало.</p>
          </div>
          <button type="button" className="btn btn-sm" onClick={() => void loadLootboxes()}>
            Обновить
          </button>
        </div>

        <p className="admin-inline-muted admin-zero-margin">{lootboxCountLabel}</p>

        {lootboxError ? <ErrorState message={lootboxError} /> : null}
        {!lootboxHistory && !lootboxError ? <LoadingState title="Загружаем открытия кейсов" /> : null}
        {lootboxHistory && lootboxHistory.length === 0 ? <p className="admin-inline-muted">Кейсы пока не открывали.</p> : null}
        {lootboxHistory && lootboxHistory.length > 0 ? (
          <div className="admin-purchase-timeline">
            {pagedLootboxHistory.map((item) => {
              const isExpanded = openedLootboxId === item.id
              const username = lootboxUsers[item.userId] ?? item.userId
              return (
                <article key={item.id} className={`admin-purchase-card${isExpanded ? ' is-expanded' : ''}`}>
                  <button
                    type="button"
                    className="admin-purchase-card-main"
                    aria-expanded={isExpanded}
                    onClick={() => setOpenedLootboxId(isExpanded ? null : item.id)}
                  >
                    <span className="admin-purchase-time">{formatDateTime(item.openedAt)}</span>
                    <span className="admin-purchase-summary">
                      <strong>{username}</strong>
                      <span>{item.lootboxAssetKey}</span>
                      <span>{formatLootboxReward(item)}</span>
                    </span>
                    <span className={`ui-badge ${item.wasCompensated ? 'ui-badge-warning' : 'ui-badge-success'}`}>
                      {item.wasCompensated ? 'Компенсация' : 'Выпало'}
                    </span>
                  </button>

                  {isExpanded ? (
                    <div className="admin-purchase-details">
                      <dl className="admin-kv admin-purchase-kv">
                        <div><dt>Игрок</dt><dd><AdminLink href={adminUserPath(item.userId)}>{username}</AdminLink></dd></div>
                        <div><dt>Кейс</dt><dd><code>{item.lootboxAssetKey}</code></dd></div>
                        <div><dt>Награда</dt><dd>{formatLootboxReward(item)}</dd></div>
                        <div><dt>Asset</dt><dd><code>{item.reward.assetKey}</code> · {item.reward.ownershipModel}</dd></div>
                        <div><dt>Открыт</dt><dd>{formatDateTime(item.openedAt)}</dd></div>
                      </dl>
                    </div>
                  ) : null}
                </article>
              )
            })}
          </div>
        ) : null}
        {lootboxHistory && lootboxHistory.length > 0 ? (
          <AdminPagination page={lootboxPage} totalPages={lootboxTotalPages} onPageChange={setLootboxPage} label="Нумерация страниц открытий кейсов" />
        ) : null}
      </section>
    </div>
  )
}
