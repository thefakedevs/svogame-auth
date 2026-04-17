import { useEffect, useState } from 'react'
import {
  getMyDefaultWalletBalance,
  getMyWalletTransactions,
  type WalletBalanceResponse,
  type WalletTransactionResponse,
} from '../api/inventory'
import { toDisplayError } from '../api/http'
import {
  getMyLootboxOpenHistory,
  listPublicLootboxes,
  type LootboxOpenHistoryResponse,
} from '../api/lootboxes'
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
  | { status: 'ready'; balance: WalletBalanceResponse; history: WalletHistoryItem[] }

type WalletHistoryItem =
  | { type: 'purchase'; id: string; createdAt: string; purchase: ShopOrderResponse }
  | { type: 'lootbox'; id: string; createdAt: string; opening: LootboxOpenHistoryResponse; lootboxName: string }
  | { type: 'wallet'; id: string; createdAt: string; transaction: WalletTransactionResponse }

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

function formatCoinDelta(value: number) {
  const sign = value > 0 ? '+' : ''
  return `${sign}${formatAmount(value)}`
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

function formatLootboxReward(opening: LootboxOpenHistoryResponse) {
  const reward = opening.reward
  if (reward.amount != null) return `${reward.displayName} x${formatAmount(reward.amount)}`
  if (reward.durationSeconds != null) return `${reward.displayName} на ${formatAmount(reward.durationSeconds)} сек.`
  return reward.displayName
}

function walletTransactionTitle(transaction: WalletTransactionResponse) {
  if (transaction.reasonText) return transaction.reasonText

  switch (transaction.operationType) {
    case 'credit':
      return 'Начисление'
    case 'debit':
      return 'Списание'
    case 'adjustment':
      return 'Корректировка баланса'
    default:
      return transaction.operationType
  }
}

function walletTransactionKind(transaction: WalletTransactionResponse) {
  switch (transaction.operationType) {
    case 'credit':
      return 'Начисление'
    case 'debit':
      return 'Списание'
    case 'adjustment':
      return 'Корректировка'
    default:
      return 'Кошелек'
  }
}

function buildWalletHistory(
  purchases: ShopOrderResponse[],
  lootboxOpenings: LootboxOpenHistoryResponse[],
  walletTransactions: WalletTransactionResponse[],
  lootboxNames: Map<string, string>,
): WalletHistoryItem[] {
  return [
    ...purchases.map((purchase): WalletHistoryItem => ({
      type: 'purchase',
      id: `purchase:${purchase.id}`,
      createdAt: purchase.createdAt,
      purchase,
    })),
    ...lootboxOpenings.map((opening): WalletHistoryItem => ({
      type: 'lootbox',
      id: `lootbox:${opening.id}`,
      createdAt: opening.openedAt,
      opening,
      lootboxName: lootboxNames.get(opening.lootboxAssetKey) ?? 'Кейс',
    })),
    ...walletTransactions.map((transaction): WalletHistoryItem => ({
      type: 'wallet',
      id: `wallet:${transaction.id}`,
      createdAt: transaction.createdAt,
      transaction,
    })),
  ].sort((left, right) => new Date(right.createdAt).getTime() - new Date(left.createdAt).getTime())
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
      const [orders, lootboxOpenings, walletTransactions, lootboxes] = await Promise.all([
        listMyShopOrders(token),
        getMyLootboxOpenHistory(token),
        getMyWalletTransactions(token, balance.currencyKey),
        listPublicLootboxes().catch(() => []),
      ])
      const lootboxNames = new Map(lootboxes.map((item) => [item.assetKey, item.assetDisplayName]))
      setState({
        status: 'ready',
        balance,
        history: buildWalletHistory(orders, lootboxOpenings, walletTransactions, lootboxNames),
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
          <h2 className="card-title">История</h2>
          <span className="ui-badge ui-badge-neutral">{state.history.length}</span>
        </div>

        {state.history.length ? (
          <div className="ownership-transaction-list">
            {state.history.map((item) => {
              if (item.type === 'purchase') {
                const purchase = item.purchase
                return (
                  <article key={item.id} className="ownership-transaction">
                    <span className="ownership-transaction-delta is-negative">
                      {formatRubPrice(purchase.totalPriceRub)}
                    </span>
                    <span className="ownership-transaction-main">
                      <strong>{purchase.productName}</strong>
                      <small>Покупка · {formatDateTime(purchase.createdAt).replace(', ', ' ')}</small>
                    </span>
                    <span className="ownership-transaction-balance ownership-transaction-status">
                      <span className={purchaseStatusClass(purchase.status)}>{purchaseStatusText(purchase.status)}</span>
                    </span>
                  </article>
                )
              }

              if (item.type === 'lootbox') {
                const opening = item.opening
                return (
                  <article key={item.id} className="ownership-transaction">
                    <span className="ownership-transaction-delta ownership-transaction-kind">
                      Кейс
                    </span>
                    <span className="ownership-transaction-main">
                      <strong>{item.lootboxName}</strong>
                      <small>Награда: {formatLootboxReward(opening)}</small>
                    </span>
                    <span className="ownership-transaction-balance ownership-transaction-status">
                      {formatDateTime(opening.openedAt).replace(', ', ' ')}
                    </span>
                  </article>
                )
              }

              const transaction = item.transaction
              return (
                <article key={item.id} className="ownership-transaction">
                  <span className={`ownership-transaction-delta ${transaction.delta < 0 ? 'is-negative' : ''}`}>
                    {formatCoinDelta(transaction.delta)}
                  </span>
                  <span className="ownership-transaction-main">
                    <strong>{walletTransactionTitle(transaction)}</strong>
                    <small>{walletTransactionKind(transaction)} · {formatDateTime(transaction.createdAt).replace(', ', ' ')}</small>
                  </span>
                  <span className="ownership-transaction-balance ownership-transaction-status">
                    Баланс: {formatAmount(transaction.balanceAfter)}
                  </span>
                </article>
              )
            })}
          </div>
        ) : (
          <p className="ownership-muted">История пока пуста.</p>
        )}
      </section>
    </main>
  )
}
