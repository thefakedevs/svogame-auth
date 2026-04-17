import { useEffect, useMemo, useState } from 'react'
import { ApiError, toDisplayError } from '../api/http'
import {
  getMyShopOrder,
  getMyShopOrderReceipt,
  getMyShopOrderReceiptPrintImage,
  type ShopOrderResponse,
  type ShopReceiptResponse,
} from '../api/shop'
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

type ReceiptState =
  | { status: 'idle' }
  | { status: 'polling'; receipt: ShopReceiptResponse | null; message: string }
  | { status: 'ready'; receipt: ShopReceiptResponse; imageUrl: string }
  | { status: 'unavailable'; receipt: ShopReceiptResponse | null; message: string }
  | { status: 'failed'; receipt: ShopReceiptResponse | null; error: string }

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
const terminalReceiptUnavailableStatuses = new Set(['not_required', 'test_payment'])

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

function receiptStatusText(receipt: ShopReceiptResponse | null) {
  if (!receipt) {
    return 'Ждем статус чека.'
  }

  switch (receipt.status) {
    case 'pending':
    case 'forming':
      return receipt.displayStatus || 'Чек формируется.'
    case 'completed':
      return receipt.displayStatus || 'Чек готов.'
    case 'failed':
      return receipt.failureProblem || receipt.displayStatus || 'Чек не удалось сформировать.'
    case 'not_required':
      return 'Для этого платежа чек не требуется.'
    case 'test_payment':
      return 'Это тестовый платеж, печатный чек недоступен.'
    default:
      return receipt.displayStatus || 'Статус чека неизвестен.'
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

function printReceiptImage(imageUrl: string) {
  const printWindow = window.open('', '_blank')
  if (!printWindow) {
    window.open(imageUrl, '_blank')
    return
  }

  printWindow.document.write(`
    <!doctype html>
    <html>
      <head>
        <title>Чек</title>
        <style>
          html, body { margin: 0; min-height: 100%; background: #fff; }
          body { display: grid; place-items: start center; padding: 24px; }
          img { max-width: 100%; height: auto; }
        </style>
      </head>
      <body>
        <img src="${imageUrl}" alt="Чек" onload="window.focus(); window.print();" />
      </body>
    </html>
  `)
  printWindow.document.close()
}

function ReceiptPanel({ state }: { state: ReceiptState }) {
  if (state.status === 'idle') {
    return null
  }

  if (state.status === 'ready') {
    return (
      <div className="shop-receipt-panel">
        <div className="shop-receipt-panel__content">
          <span className="ui-badge ui-badge-success">Чек готов</span>
          <h2>Чек по заказу</h2>
          <p>{receiptStatusText(state.receipt)}</p>
        </div>
        <img className="shop-receipt-panel__image" src={state.imageUrl} alt="Чек по заказу" />
        <div className="shop-callback-actions">
          <a className="btn primary" href={state.imageUrl} download={`receipt-${state.receipt.id}.png`}>
            Скачать чек
          </a>
          <button className="btn" type="button" onClick={() => printReceiptImage(state.imageUrl)}>
            Распечатать чек
          </button>
        </div>
      </div>
    )
  }

  if (state.status === 'failed') {
    return (
      <div className="shop-receipt-panel">
        <div className="shop-receipt-panel__content">
          <span className="ui-badge ui-badge-warning">Чек</span>
          <h2>Чек пока недоступен</h2>
          <p>{state.error}</p>
        </div>
      </div>
    )
  }

  if (state.status === 'unavailable') {
    return (
      <div className="shop-receipt-panel">
        <div className="shop-receipt-panel__content">
          <span className="ui-badge">Чек</span>
          <h2>Печатный чек недоступен</h2>
          <p>{state.message}</p>
        </div>
      </div>
    )
  }

  return (
    <div className="shop-receipt-panel">
      <div className="shop-receipt-panel__content">
        <span className="ui-badge">Чек</span>
        <h2>Готовим чек</h2>
        <p>{state.message || receiptStatusText(state.receipt)}</p>
      </div>
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
  const [receiptState, setReceiptState] = useState<ReceiptState>({ status: 'idle' })
  const [isTabVisible, setIsTabVisible] = useState(() => {
    if (typeof document === 'undefined') {
      return true
    }
    return document.visibilityState === 'visible'
  })

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

  useEffect(() => {
    const handleVisibilityChange = () => {
      setIsTabVisible(document.visibilityState === 'visible')
    }

    document.addEventListener('visibilitychange', handleVisibilityChange)

    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange)
    }
  }, [])

  useEffect(() => {
    if (state.status !== 'success') {
      return
    }

    const token = getAuthToken()
    if (!token) {
      return
    }

    let cancelled = false
    let timer: number | undefined
    let imageUrl: string | undefined

    const scheduleNextPoll = () => {
      timer = window.setTimeout(poll, 5000)
    }

    const poll = async () => {
      try {
        const receipt = await getMyShopOrderReceipt(token, state.order.id)
        if (cancelled) return

        if (receipt.status === 'completed') {
          setReceiptState({
            status: 'polling',
            receipt,
            message: 'Загружаем изображение чека.',
          })

          try {
            const image = await getMyShopOrderReceiptPrintImage(token, state.order.id)
            if (cancelled) return

            imageUrl = URL.createObjectURL(image)
            setReceiptState({ status: 'ready', receipt, imageUrl })
          } catch (cause) {
            if (cancelled) return

            setReceiptState({
              status: 'unavailable',
              receipt,
              message: toDisplayError(cause, 'Чек готов, но изображение для печати пока недоступно.'),
            })
          }
          return
        }

        if (receipt.status === 'failed') {
          setReceiptState({
            status: 'failed',
            receipt,
            error: receiptStatusText(receipt),
          })
          return
        }

        if (terminalReceiptUnavailableStatuses.has(receipt.status)) {
          setReceiptState({
            status: 'unavailable',
            receipt,
            message: receiptStatusText(receipt),
          })
          return
        }

        setReceiptState({
          status: 'polling',
          receipt,
          message: receiptStatusText(receipt),
        })
        scheduleNextPoll()
      } catch (cause) {
        if (cancelled) return

        if (cause instanceof ApiError && cause.status === 404) {
          setReceiptState({
            status: 'polling',
            receipt: null,
            message: 'Чек еще не появился. Проверим еще раз через несколько секунд.',
          })
          scheduleNextPoll()
          return
        }

        setReceiptState({
          status: 'failed',
          receipt: null,
          error: toDisplayError(cause, 'Не удалось проверить статус чека.'),
        })
      }
    }

    queueMicrotask(() => {
      if (!cancelled) {
        setReceiptState({
          status: 'polling',
          receipt: state.order.receipt ?? null,
          message: receiptStatusText(state.order.receipt ?? null),
        })
      }
    })
    void poll()

    return () => {
      cancelled = true
      if (timer) window.clearTimeout(timer)
      if (imageUrl) URL.revokeObjectURL(imageUrl)
    }
  }, [state])

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

  if (state.status === 'success') {
    return (
      <main className="page ownership-page shop-page">
        <section className="card shop-callback-card">
          <span className="ui-badge ui-badge-success">Оплачено</span>
          <h1 className="card-title">Скин добавлен в инвентарь</h1>
          <p className="card-text">{statusText(state.order.status)}</p>
          <OrderSummary order={state.order} />
          <ReceiptPanel state={receiptState} />
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
