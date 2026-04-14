import { useEffect, useState } from 'react'
import {
  getMyDefaultWalletBalance,
  type WalletBalanceResponse,
} from '../api/inventory'
import { toDisplayError } from '../api/http'
import { listMyShopOrders, type ShopOrderResponse } from '../api/shop'
import ErrorState from '../components/ErrorState'
import LoadingState from '../components/LoadingState'
import { currentAppPath, redirectToAuth } from '../routes/auth'
import { getAuthToken } from '../shared/session/auth-session'
import './OwnershipPage.css'

type WalletState =
  | { status: 'loading' }
  | { status: 'unauthorized' }
  | { status: 'error'; error: string }
  | { status: 'ready'; balance: WalletBalanceResponse; purchases: ShopOrderResponse[] }

const numberFormatter = new Intl.NumberFormat('ru-RU')
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

function formatAmount(value: number) {
  return numberFormatter.format(value)
}

function formatRubPrice(value: number) {
  return `${formatAmount(Math.abs(value))} ₽`
}

function purchaseStatusText(status: string) {
  switch (status) {
    case 'pending_payment':
      return 'Ожидает оплаты'
    case 'paid':
      return 'Оплачено'
    case 'fulfillment_in_progress':
      return 'Выдаем скин'
    case 'fulfilled':
      return 'Завершено'
    case 'payment_canceled':
      return 'Платеж отменен'
    case 'payment_expired':
      return 'Оплата истекла'
    case 'payment_validation_failed':
      return 'Ошибка проверки платежа'
    case 'fulfillment_failed':
      return 'Ошибка выдачи'
    default:
      return status
  }
}

function purchaseStatusClass(status: string) {
  if (status === 'fulfilled') return 'ui-badge ui-badge-success'
  if (status === 'pending_payment' || status === 'paid' || status === 'fulfillment_in_progress') return 'ui-badge ui-badge-neutral'
  return 'ui-badge ui-badge-warning'
}

export default function WalletPage() {
  const [state, setState] = useState<WalletState>({ status: 'loading' })

  const load = async () => {
    const token = getAuthToken()
    if (!token) {
      setState({ status: 'unauthorized' })
      return
    }

    setState({ status: 'loading' })
    try {
      const [balance, orders] = await Promise.all([
        getMyDefaultWalletBalance(token),
        listMyShopOrders(token),
      ])
      setState({
        status: 'ready',
        balance,
        purchases: orders.slice().sort((left, right) => new Date(right.createdAt).getTime() - new Date(left.createdAt).getTime()),
      })
    } catch (cause) {
      setState({ status: 'error', error: toDisplayError(cause, 'Не удалось загрузить кошелек.') })
    }
  }

  useEffect(() => {
    queueMicrotask(() => void load())
  }, [])

  if (state.status === 'loading') {
    return <LoadingState title="Загружаем кошелек" />
  }

  if (state.status === 'unauthorized') {
    return (
      <section className="card profile-state-card">
        <span className="ui-badge ui-badge-warning">Доступ</span>
        <h1 className="card-title">Нужно войти</h1>
        <p className="card-text">Кошелек доступен только после входа через Discord.</p>
        <button className="btn primary" type="button" onClick={() => redirectToAuth(currentAppPath())}>
          Войти
        </button>
      </section>
    )
  }

  if (state.status === 'error') {
    return <ErrorState title="Кошелек недоступен" message={state.error} primaryActionLabel="Повторить" onPrimaryAction={() => void load()} />
  }

  return (
    <main className="page ownership-page">
      <section className="card ownership-wallet-default-card">
        <h1 className="card-title">Кошелек</h1>
        <hr />
        <strong>{formatAmount(state.balance.balance)} защеккоинов</strong>
      </section>

      <section className="card ownership-section">
        <div className="ownership-section-head">
          <h2 className="card-title">Покупки</h2>
          <span className="ui-badge ui-badge-neutral">{state.purchases.length}</span>
        </div>

        {state.purchases.length ? (
          <div className="ownership-transaction-list">
            {state.purchases.map((purchase) => (
              <article key={purchase.id} className="ownership-transaction">
                <span className="ownership-transaction-delta is-negative">
                  {formatRubPrice(purchase.totalPriceRub)}
                </span>
                <span className="ownership-transaction-main">
                  <strong>{purchase.productName}</strong>
                  <small>{formatDateTime(purchase.createdAt).replace(', ', ' ')}</small>
                </span>
                <span className="ownership-transaction-balance ownership-transaction-status">
                  <span className={purchaseStatusClass(purchase.status)}>{purchaseStatusText(purchase.status)}</span>
                </span>
              </article>
            ))}
          </div>
        ) : (
          <p className="ownership-muted">Покупок пока нет.</p>
        )}
      </section>
    </main>
  )
}
