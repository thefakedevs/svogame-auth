import { useEffect, useMemo, useState } from 'react'
import { toDisplayError } from '../api/http'
import { getMyShopOrder, type ShopOrderResponse } from '../api/shop'
import ErrorState from '../components/ErrorState'
import LoadingState from '../components/LoadingState'
import { currentAppPath, redirectToAuth } from '../routes/auth'
import { paths } from '../routes/paths'
import { useQueryParams } from '../shared/navigation/query'
import { getAuthToken } from '../shared/session/auth-session'
import Pride from 'react-canvas-confetti/dist/presets/pride'
import './OwnershipPage.css'
import './ShopPage.css'

type CallbackState =
  | { status: 'polling'; order: ShopOrderResponse | null }
  | { status: 'success'; order: ShopOrderResponse }
  | { status: 'failed'; order: ShopOrderResponse; error: string }
  | { status: 'error'; error: string }

const priceFormatter = new Intl.NumberFormat('ru-RU', {
  style: 'currency',
  currency: 'RUB',
  maximumFractionDigits: 0,
})

const terminalSuccessStatuses = new Set(['fulfilled'])
const terminalFailureStatuses = new Set([
  'fulfillment_failed',
  'payment_canceled',
  'payment_expired',
  'payment_validation_failed',
])

function formatPrice(value: number) {
  return priceFormatter.format(value)
}

function statusText(status: string) {
  switch (status) {
    case 'pending_payment':
      return 'Ждем подтверждение оплаты.'
    case 'paid':
      return 'Оплата прошла, выдаем скин.'
    case 'fulfillment_in_progress':
      return 'Выдаем скин в инвентарь.'
    case 'fulfilled':
      return 'Покупка завершена.'
    case 'payment_canceled':
      return 'Платеж отменен.'
    case 'payment_expired':
      return 'Время оплаты истекло.'
    case 'payment_validation_failed':
      return 'Платеж не прошел проверку.'
    case 'fulfillment_failed':
      return 'Оплата прошла, но скин не удалось выдать.'
    default:
      return status
  }
}

function OrderSummary({ order }: { order: ShopOrderResponse }) {
  return (
    <div className="shop-callback-summary">
      <strong>{order.productName}</strong>
      <span>{formatPrice(order.totalPriceRub)}</span>
      <small>Заказ {order.id}</small>
    </div>
  )
}

function CallbackActions() {
  return (
    <div className="shop-callback-actions">
      <a className="btn primary" href={paths.shop}>Продолжить покупки</a>
      <a className="btn" href={paths.inventory}>Перейти в инвентарь</a>
    </div>
  )
}

export default function ShopCheckoutReturnPage() {
  const query = useQueryParams()
  const orderId = useMemo(() => query.get('orderId')?.trim() ?? '', [query])
  const [state, setState] = useState<CallbackState>({ status: 'polling', order: null })

  useEffect(() => {
    const token = getAuthToken()
    if (!orderId || !token) {
      return
    }

    let cancelled = false
    let timer: number | undefined

    const poll = async () => {
      try {
        const order = await getMyShopOrder(token, orderId)
        if (cancelled) return

        if (terminalSuccessStatuses.has(order.status)) {
          setState({ status: 'success', order })
          return
        }

        if (terminalFailureStatuses.has(order.status)) {
          setState({
            status: 'failed',
            order,
            error: order.failureProblem ?? statusText(order.status),
          })
          return
        }

        setState({ status: 'polling', order })
        timer = window.setTimeout(poll, 5000)
      } catch (cause) {
        if (!cancelled) {
          setState({ status: 'error', error: toDisplayError(cause, 'Не удалось проверить заказ.') })
        }
      }
    }

    queueMicrotask(() => {
      if (!cancelled) setState({ status: 'polling', order: null })
    })
    void poll()

    return () => {
      cancelled = true
      if (timer) window.clearTimeout(timer)
    }
  }, [orderId])

  if (!orderId) {
    return (
      <main className="page ownership-page shop-page">
        <ErrorState title="Заказ не найден" message="В ссылке возврата нет orderId." primaryActionLabel="В магазин" onPrimaryAction={() => window.location.assign(paths.shop)} />
      </main>
    )
  }

  if (!getAuthToken()) {
    return (
      <main className="page ownership-page shop-page">
        <section className="card profile-state-card">
          <span className="ui-badge ui-badge-warning">Доступ</span>
          <h1 className="card-title">Нужно войти</h1>
          <p className="card-text">Войдите в аккаунт, чтобы проверить статус заказа.</p>
          <button className="btn primary" type="button" onClick={() => redirectToAuth(currentAppPath())}>
            Войти
          </button>
        </section>
      </main>
    )
  }

  if (state.status === 'error') {
    return (
      <main className="page ownership-page shop-page">
        <ErrorState title="Статус заказа недоступен" message={state.error} primaryActionLabel="Повторить" onPrimaryAction={() => window.location.reload()} />
      </main>
    )
  }

  // 1. Track if the tab is currently focused/visible
  const [isTabVisible, setIsTabVisible] = useState(true);

  useEffect(() => {
    const handleVisibilityChange = () => {
      setIsTabVisible(document.visibilityState === 'visible');
    };

    // Add event listener for tab switching
    document.addEventListener('visibilitychange', handleVisibilityChange);

    // Cleanup listener on unmount
    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange);
    };
  }, []);

  if (state.status === 'success') {
    return (
      <main className="page ownership-page shop-page">
        <section className="card shop-callback-card">
          <span className="ui-badge ui-badge-success">Оплачено</span>
          <h1 className="card-title">Скин добавлен в инвентарь</h1>
          <p className="card-text">{statusText(state.order.status)}</p>
          <OrderSummary order={state.order} />
          <CallbackActions />
        </section>
        {isTabVisible && (
          <>
            <Pride
              style={{ position: 'fixed', pointerEvents: 'none', width: '100%', height: '100%', top: 0, left: 0, zIndex: -1 }}
              autorun={{ speed: 1 }}
              decorateOptions={(defaultOptions) => ({
                ...defaultOptions,
                particleCount: 50,
                spread: 90,
                zIndex: -1,
                colors: ['#26ccff', '#a25afd', '#ff5e7e', '#88ff5a', '#fcff42', '#ffa62d', '#ff36ff']
              })}
            />
          </>
        )}
      </main>
    )
  }

  if (state.status === 'failed') {
    return (
      <main className="page ownership-page shop-page">
        <section className="card shop-callback-card">
          <span className="ui-badge ui-badge-warning">Ошибка оплаты</span>
          <h1 className="card-title">Покупка не завершена</h1>
          <p className="card-text">{state.error}</p>
          <OrderSummary order={state.order} />
          <CallbackActions />
        </section>
      </main>
    )
  }

  return (
    <main className="page ownership-page shop-page">
      <LoadingState title="Проверяем оплату" message={state.order ? statusText(state.order.status) : 'Запрашиваем статус заказа.'}>
        {state.order ? <OrderSummary order={state.order} /> : null}
      </LoadingState>
    </main>
  )
}
