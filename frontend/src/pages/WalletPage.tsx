import { useEffect, useState } from 'react'
import {
  getMyDefaultWalletBalance,
  getMyWalletTransactions,
  type WalletBalanceResponse,
  type WalletTransactionResponse,
} from '../api/inventory'
import { toDisplayError } from '../api/http'
import ErrorState from '../components/ErrorState'
import LoadingState from '../components/LoadingState'
import { currentAppPath, redirectToAuth } from '../routes/auth'
import { getAuthToken } from '../shared/session/auth-session'
import './OwnershipPage.css'

type WalletState =
  | { status: 'loading' }
  | { status: 'unauthorized' }
  | { status: 'error'; error: string }
  | { status: 'ready'; balance: WalletBalanceResponse; purchases: WalletTransactionResponse[] }

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

function formatPrice(value: number) {
  return `${formatAmount(Math.abs(value))} защекинов`
}

function purchaseTitle(purchase: WalletTransactionResponse) {
  if (purchase.reasonText) return purchase.reasonText
  if (purchase.reasonCode) return purchase.reasonCode
  return 'Покупка'
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
      const balance = await getMyDefaultWalletBalance(token)
      const transactions = await getMyWalletTransactions(token, balance.currencyKey)
      setState({
        status: 'ready',
        balance,
        purchases: transactions.filter((transaction) => transaction.operationType.toLowerCase() === 'purchase'),
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
        <strong>{formatAmount(state.balance.balance)} защекинов</strong>
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
                  {formatPrice(purchase.delta)}
                </span>
                <span className="ownership-transaction-main">
                  <strong>{purchaseTitle(purchase)}</strong>
                  <small>{formatDateTime(purchase.createdAt)}</small>
                </span>
                <span className="ownership-transaction-balance">
                  После: {formatAmount(purchase.balanceAfter)} защекинов
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
