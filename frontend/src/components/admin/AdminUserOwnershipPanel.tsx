import { useCallback, useEffect, useMemo, useState } from 'react'
import type { ReactNode } from 'react'
import toast from 'react-hot-toast'
import { toDisplayError } from '../../api/http'
import {
  addAdminStackable,
  adjustAdminDefaultWallet,
  creditAdminDefaultWallet,
  debitAdminDefaultWallet,
  getAdminUserDefaultWalletBalance,
  getAdminUserInventory,
  getAdminUserInventoryHistory,
  getAdminUserWalletTransactions,
  grantAdminEntitlement,
  listAllAdminAssets,
  prolongAdminExpirable,
  removeAdminStackable,
  revokeAdminEntitlement,
  revokeAdminExpirable,
  setAdminExpirableExpiration,
  setAdminStackable,
  type AssetResponse,
  type InventoryOperationResponse,
  type InventoryResponse,
  type WalletBalanceResponse,
  type WalletTransactionResponse,
} from '../../api/inventory'
import AppPortal from '../../shared/ui/portal/AppPortal'
import AdminAssetImage from './AdminAssetImage'
import AdminAssetSelect from './AdminAssetSelect'

type AdminOwnershipState =
  | { status: 'loading' }
  | { status: 'error'; error: string }
  | {
    status: 'ready'
    assets: AssetResponse[]
    inventory: InventoryResponse
    wallet: WalletBalanceResponse
    walletTransactions: WalletTransactionResponse[]
    inventoryHistory: InventoryOperationResponse[]
  }

type ModalKind = 'wallet' | 'entitlements' | 'stackables' | 'expirables' | 'purchases' | 'inventoryHistory' | null

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
  return Number.isNaN(date.getTime()) ? value : dateTimeFormatter.format(date).replace(', ', ' ')
}

function formatAmount(value: number) {
  return numberFormatter.format(value)
}

function makeAssetMap(assets: AssetResponse[]) {
  const map = new Map<string, AssetResponse>()
  for (const asset of assets) {
    map.set(asset.key, asset)
    map.set(asset.id, asset)
  }
  return map
}

function assetByKey(assetMap: Map<string, AssetResponse>, assetKey: string, assetDefinitionId?: string) {
  return assetMap.get(assetKey) ?? (assetDefinitionId ? assetMap.get(assetDefinitionId) : null) ?? null
}

function assetName(assetMap: Map<string, AssetResponse>, assetKey: string, assetDefinitionId?: string) {
  return assetByKey(assetMap, assetKey, assetDefinitionId)?.displayName ?? assetKey
}

function parseAmount(value: string) {
  const amount = Number(value)
  return Number.isFinite(amount) ? Math.trunc(amount) : null
}

function toIsoFromLocal(value: string) {
  if (!value) return null
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? null : date.toISOString()
}

export default function AdminUserOwnershipPanel({ token, userId }: { token: string; userId: string }) {
  const [state, setState] = useState<AdminOwnershipState>({ status: 'loading' })
  const [activeModal, setActiveModal] = useState<ModalKind>(null)
  const [selectedEntitlementKey, setSelectedEntitlementKey] = useState('')
  const [selectedStackableKey, setSelectedStackableKey] = useState('')
  const [selectedExpirableKey, setSelectedExpirableKey] = useState('')
  const [stackableAmount, setStackableAmount] = useState('1')
  const [durationDays, setDurationDays] = useState('30')
  const [expiresAt, setExpiresAt] = useState('')
  const [reasonText, setReasonText] = useState('')
  const [walletAmount, setWalletAmount] = useState('0')
  const [walletReasonText, setWalletReasonText] = useState('')
  const [isMutating, setIsMutating] = useState(false)

  const load = useCallback(async () => {
    setState({ status: 'loading' })
    try {
      const [assets, inventory, wallet, inventoryHistory] = await Promise.all([
        listAllAdminAssets(token),
        getAdminUserInventory(token, userId),
        getAdminUserDefaultWalletBalance(token, userId),
        getAdminUserInventoryHistory(token, userId),
      ])
      const walletTransactions = await getAdminUserWalletTransactions(token, userId, wallet.currencyKey)
      setState({ status: 'ready', assets, inventory, wallet, walletTransactions, inventoryHistory })
    } catch (cause) {
      setState({ status: 'error', error: toDisplayError(cause, 'Не удалось загрузить инвентарь и кошелек.') })
    }
  }, [token, userId])

  useEffect(() => {
    queueMicrotask(() => void load())
  }, [load])

  const assetMap = useMemo(() => {
    if (state.status !== 'ready') return new Map<string, AssetResponse>()
    return makeAssetMap(state.assets)
  }, [state])

  const entitlementAssets = useMemo(() => {
    if (state.status !== 'ready') return []
    return state.assets.filter((asset) => !asset.isCurrency && asset.ownershipModel === 'entitlement')
  }, [state])

  const stackableAssets = useMemo(() => {
    if (state.status !== 'ready') return []
    return state.assets.filter((asset) => !asset.isCurrency && asset.ownershipModel === 'stackable')
  }, [state])

  const expirableAssets = useMemo(() => {
    if (state.status !== 'ready') return []
    return state.assets.filter((asset) => !asset.isCurrency && asset.ownershipModel === 'expirable')
  }, [state])

  const selectedEntitlement = entitlementAssets.find((asset) => asset.key === selectedEntitlementKey) ?? null
  const selectedStackable = stackableAssets.find((asset) => asset.key === selectedStackableKey) ?? null
  const selectedExpirable = expirableAssets.find((asset) => asset.key === selectedExpirableKey) ?? null

  const inventoryMutationBody = (extra?: { amount?: number | null; durationSeconds?: number | null; expiresAt?: string | null }) => ({
    ...extra,
    reasonCode: 'admin_panel',
    reasonText: reasonText.trim() || null,
  })

  const runMutation = async (action: () => Promise<unknown>, successMessage: string) => {
    if (isMutating) return
    setIsMutating(true)
    try {
      await action()
      toast.success(successMessage)
      await load()
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось выполнить операцию.'))
    } finally {
      setIsMutating(false)
    }
  }

  const mutateWallet = async (kind: 'credit' | 'debit' | 'adjust') => {
    const amount = parseAmount(walletAmount)
    if (amount === null || amount < 0 || (kind !== 'adjust' && amount <= 0)) {
      toast.error(kind === 'adjust' ? 'Укажите неотрицательный баланс.' : 'Укажите положительную сумму.')
      return
    }

    const body = {
      amount,
      reasonCode: 'admin_panel',
      reasonText: walletReasonText.trim() || null,
    }

    await runMutation(async () => {
      if (kind === 'credit') return creditAdminDefaultWallet(token, userId, body)
      if (kind === 'debit') return debitAdminDefaultWallet(token, userId, body)
      return adjustAdminDefaultWallet(token, userId, body)
    }, 'Баланс защекоинов обновлен.')
  }

  const mutateEntitlement = async (action: 'grant' | 'revoke', assetKey?: string) => {
    const key = assetKey ?? selectedEntitlement?.key
    if (!key) {
      toast.error('Выберите скин.')
      return
    }

    await runMutation(async () => {
      if (action === 'grant') return grantAdminEntitlement(token, userId, key, inventoryMutationBody())
      return revokeAdminEntitlement(token, userId, key, inventoryMutationBody())
    }, action === 'grant' ? 'Скин выдан.' : 'Скин снят.')
  }

  const mutateStackable = async (action: 'add' | 'remove' | 'set' | 'clear', assetKey?: string) => {
    const key = assetKey ?? selectedStackable?.key
    if (!key) {
      toast.error('Выберите предмет с количеством.')
      return
    }

    const amount = action === 'clear' ? 0 : parseAmount(stackableAmount)
    if (amount === null || amount < 0 || (action !== 'set' && action !== 'clear' && amount <= 0)) {
      toast.error(action === 'set' ? 'Укажите неотрицательное количество.' : 'Укажите положительное количество.')
      return
    }

    await runMutation(async () => {
      if (action === 'add') return addAdminStackable(token, userId, key, inventoryMutationBody({ amount }))
      if (action === 'remove') return removeAdminStackable(token, userId, key, inventoryMutationBody({ amount }))
      return setAdminStackable(token, userId, key, inventoryMutationBody({ amount }))
    }, action === 'clear' ? 'Предмет обнулен.' : 'Предмет обновлен.')
  }

  const mutateExpirable = async (action: 'prolong' | 'expiration' | 'revoke', assetKey?: string) => {
    const key = assetKey ?? selectedExpirable?.key
    if (!key) {
      toast.error('Выберите временный бонус.')
      return
    }

    if (action === 'prolong') {
      const days = parseAmount(durationDays)
      if (days === null || days <= 0) {
        toast.error('Укажите положительное количество дней.')
        return
      }
      await runMutation(
        async () => prolongAdminExpirable(token, userId, key, inventoryMutationBody({ durationSeconds: days * 24 * 60 * 60 })),
        'Временный бонус продлен.',
      )
      return
    }

    if (action === 'expiration') {
      const iso = toIsoFromLocal(expiresAt)
      if (!iso) {
        toast.error('Укажите дату окончания.')
        return
      }
      await runMutation(
        async () => setAdminExpirableExpiration(token, userId, key, inventoryMutationBody({ expiresAt: iso })),
        'Срок временного бонуса обновлен.',
      )
      return
    }

    await runMutation(async () => revokeAdminExpirable(token, userId, key, inventoryMutationBody()), 'Временный бонус снят.')
  }

  if (state.status === 'loading') {
    return (
      <section className="admin-card-subsection">
        <h3 className="card-title">Инвентарь и кошелек</h3>
        <div className="admin-ownership-actions">
          <button type="button" className="btn btn-sm" disabled>Загружаем...</button>
        </div>
      </section>
    )
  }

  if (state.status === 'error') {
    return (
      <section className="admin-card-subsection">
        <h3 className="card-title">Инвентарь и кошелек</h3>
        <p className="admin-inline-muted">{state.error}</p>
        <button type="button" className="btn btn-sm" onClick={() => void load()}>
          Повторить
        </button>
      </section>
    )
  }

  const purchases = state.walletTransactions.filter((transaction) => transaction.operationType.toLowerCase() === 'purchase')

  return (
    <section className="admin-card-subsection">
      <div className="admin-ownership-summary-head">
        <h3 className="card-title">Инвентарь и кошелек</h3>
        <button type="button" className="btn btn-sm" disabled={isMutating} onClick={() => void load()}>
          Обновить
        </button>
      </div>

      <div className="admin-user-ownership-actions">
        <button type="button" className="btn btn-sm" onClick={() => setActiveModal('wallet')}>
          Баланс: {formatAmount(state.wallet.balance)} защекоинов
        </button>
        <button type="button" className="btn btn-sm" onClick={() => setActiveModal('entitlements')}>
          Скины: {state.inventory.entitlements.length}
        </button>
        <button type="button" className="btn btn-sm" onClick={() => setActiveModal('stackables')}>
          Предметы: {state.inventory.stackables.length}
        </button>
        <button type="button" className="btn btn-sm" onClick={() => setActiveModal('expirables')}>
          Временные: {state.inventory.expirables.length}
        </button>
        <button type="button" className="btn btn-sm" onClick={() => setActiveModal('purchases')}>
          Покупки: {purchases.length}
        </button>
        <button type="button" className="btn btn-sm" onClick={() => setActiveModal('inventoryHistory')}>
          История инвентаря: {state.inventoryHistory.length}
        </button>
      </div>

      {activeModal === 'wallet' ? (
        <AdminOwnershipModal title="Баланс игрока" titleId="admin-wallet-modal-title" isBusy={isMutating} onClose={() => setActiveModal(null)}>
          <div className="admin-ownership-modal-summary">
            <strong>{formatAmount(state.wallet.balance)} защекоинов</strong>
            <small>Обновлено: {formatDateTime(state.wallet.updatedAt)}</small>
          </div>
          <div className="admin-ownership-form">
            <input className="ui-input" inputMode="numeric" value={walletAmount} onChange={(event) => setWalletAmount(event.target.value)} placeholder="Сумма" />
            <input className="ui-input" value={walletReasonText} onChange={(event) => setWalletReasonText(event.target.value)} placeholder="Причина" />
          </div>
          <div className="ui-modal-footer admin-wallet-modal-footer">
            <button className="btn" type="button" disabled={isMutating} onClick={() => setActiveModal(null)}>Закрыть</button>
            <button className="btn" type="button" disabled={isMutating} onClick={() => void mutateWallet('adjust')}>Выставить</button>
            <button className="btn danger" type="button" disabled={isMutating} onClick={() => void mutateWallet('debit')}>Списать</button>
            <button className="btn primary" type="button" disabled={isMutating} onClick={() => void mutateWallet('credit')}>Пополнить</button>
          </div>
        </AdminOwnershipModal>
      ) : null}

      {activeModal === 'entitlements' ? (
        <AdminOwnershipModal title="Скины игрока" titleId="admin-entitlements-modal-title" isBusy={isMutating} onClose={() => setActiveModal(null)}>
          <InventoryGrantForm
            token={token}
            assets={entitlementAssets}
            selectedKey={selectedEntitlement?.key ?? ''}
            reasonText={reasonText}
            onSelectedKeyChange={setSelectedEntitlementKey}
            onReasonTextChange={setReasonText}
            onGrant={() => void mutateEntitlement('grant')}
            disabled={isMutating}
            emptyText="Скинов в каталоге нет."
          />
          <AdminInventoryModalList
            token={token}
            title="Выдано"
            items={state.inventory.entitlements.map((item) => ({
              key: item.assetKey,
              title: assetName(assetMap, item.assetKey, item.assetDefinitionId),
              asset: assetByKey(assetMap, item.assetKey, item.assetDefinitionId),
              meta: `Выдано: ${formatDateTime(item.grantedAt)}`,
              onRemove: () => void mutateEntitlement('revoke', item.assetKey),
            }))}
            removeLabel="Снять"
            emptyText="У игрока нет скинов."
            disabled={isMutating}
          />
        </AdminOwnershipModal>
      ) : null}

      {activeModal === 'stackables' ? (
        <AdminOwnershipModal title="Предметы с количеством" titleId="admin-stackables-modal-title" isBusy={isMutating} onClose={() => setActiveModal(null)}>
          <StackableControlForm
            token={token}
            assets={stackableAssets}
            selectedKey={selectedStackable?.key ?? ''}
            amount={stackableAmount}
            reasonText={reasonText}
            disabled={isMutating}
            onSelectedKeyChange={setSelectedStackableKey}
            onAmountChange={setStackableAmount}
            onReasonTextChange={setReasonText}
            onAdd={() => void mutateStackable('add')}
            onRemove={() => void mutateStackable('remove')}
            onSet={() => void mutateStackable('set')}
          />
          <AdminInventoryModalList
            token={token}
            title="На балансе"
            items={state.inventory.stackables.map((item) => ({
              key: item.assetKey,
              title: assetName(assetMap, item.assetKey, item.assetDefinitionId),
              asset: assetByKey(assetMap, item.assetKey, item.assetDefinitionId),
              meta: `${formatAmount(item.amount)} · ${formatDateTime(item.updatedAt)}`,
              onRemove: () => void mutateStackable('clear', item.assetKey),
            }))}
            removeLabel="Обнулить"
            emptyText="У игрока нет предметов с количеством."
            disabled={isMutating}
          />
        </AdminOwnershipModal>
      ) : null}

      {activeModal === 'expirables' ? (
        <AdminOwnershipModal title="Временные бонусы" titleId="admin-expirables-modal-title" isBusy={isMutating} onClose={() => setActiveModal(null)}>
          <ExpirableControlForm
            token={token}
            assets={expirableAssets}
            selectedKey={selectedExpirable?.key ?? ''}
            durationDays={durationDays}
            expiresAt={expiresAt}
            reasonText={reasonText}
            disabled={isMutating}
            onSelectedKeyChange={setSelectedExpirableKey}
            onDurationDaysChange={setDurationDays}
            onExpiresAtChange={setExpiresAt}
            onReasonTextChange={setReasonText}
            onProlong={() => void mutateExpirable('prolong')}
            onSetExpiration={() => void mutateExpirable('expiration')}
          />
          <AdminInventoryModalList
            token={token}
            title="Активно"
            items={state.inventory.expirables.map((item) => ({
              key: item.assetKey,
              title: assetName(assetMap, item.assetKey, item.assetDefinitionId),
              asset: assetByKey(assetMap, item.assetKey, item.assetDefinitionId),
              meta: `До: ${formatDateTime(item.expiresAt)}`,
              onRemove: () => void mutateExpirable('revoke', item.assetKey),
            }))}
            removeLabel="Снять"
            emptyText="У игрока нет временных бонусов."
            disabled={isMutating}
          />
        </AdminOwnershipModal>
      ) : null}

      {activeModal === 'purchases' ? (
        <AdminOwnershipModal title="История покупок" titleId="admin-purchases-modal-title" isBusy={isMutating} onClose={() => setActiveModal(null)}>
          <AdminHistoryModalList
            items={purchases.map((item) => ({
              id: item.id,
              title: item.reasonText || item.reasonCode || 'Покупка',
              meta: `${formatAmount(Math.abs(item.delta))} защекоинов · после: ${formatAmount(item.balanceAfter)} · ${formatDateTime(item.createdAt)}`,
            }))}
            emptyText="Покупок нет."
          />
        </AdminOwnershipModal>
      ) : null}

      {activeModal === 'inventoryHistory' ? (
        <AdminOwnershipModal title="История инвентаря" titleId="admin-inventory-history-modal-title" isBusy={isMutating} onClose={() => setActiveModal(null)}>
          <AdminHistoryModalList
            items={state.inventoryHistory.map((item) => ({
              id: item.id,
              title: `${item.operationType} · ${assetName(assetMap, item.assetKey, item.assetDefinitionId)}`,
              meta: `${item.reasonText || item.reasonCode || item.actorKind} · ${formatDateTime(item.createdAt)}`,
            }))}
            emptyText="Записей истории нет."
          />
        </AdminOwnershipModal>
      ) : null}
    </section>
  )
}

function AdminOwnershipModal({
  title,
  titleId,
  isBusy,
  onClose,
  children,
}: {
  title: string
  titleId: string
  isBusy: boolean
  onClose: () => void
  children: ReactNode
}) {
  return (
    <AppPortal>
      <div className="ui-modal-backdrop" role="presentation" onClick={() => !isBusy && onClose()}>
        <div className="ui-modal admin-ownership-modal" role="dialog" aria-modal="true" aria-labelledby={titleId} onClick={(event) => event.stopPropagation()}>
          <div className="ui-modal-header">
            <h2 id={titleId} className="ui-modal-title">{title}</h2>
            <button className="ui-modal-close" type="button" aria-label="Закрыть" disabled={isBusy} onClick={onClose}>
              ×
            </button>
          </div>
          <div className="ui-modal-body admin-ownership-modal-body">
            {children}
          </div>
        </div>
      </div>
    </AppPortal>
  )
}

function InventoryGrantForm({
  token,
  assets,
  selectedKey,
  reasonText,
  disabled,
  emptyText,
  onSelectedKeyChange,
  onReasonTextChange,
  onGrant,
}: {
  token: string
  assets: AssetResponse[]
  selectedKey: string
  reasonText: string
  disabled: boolean
  emptyText: string
  onSelectedKeyChange: (value: string) => void
  onReasonTextChange: (value: string) => void
  onGrant: () => void
}) {
  if (!assets.length) return <p className="admin-inline-muted">{emptyText}</p>

  return (
    <div className="admin-ownership-form">
      <AdminAssetSelect token={token} assets={assets} value={selectedKey} disabled={disabled} onChange={(assetKey) => onSelectedKeyChange(assetKey)} />
      <input className="ui-input" value={reasonText} disabled={disabled} onChange={(event) => onReasonTextChange(event.target.value)} placeholder="Причина операции" />
      <div className="admin-ownership-actions">
        <button className="btn btn-sm primary" type="button" disabled={disabled} onClick={onGrant}>Выдать</button>
      </div>
    </div>
  )
}

function StackableControlForm({
  token,
  assets,
  selectedKey,
  amount,
  reasonText,
  disabled,
  onSelectedKeyChange,
  onAmountChange,
  onReasonTextChange,
  onAdd,
  onRemove,
  onSet,
}: {
  token: string
  assets: AssetResponse[]
  selectedKey: string
  amount: string
  reasonText: string
  disabled: boolean
  onSelectedKeyChange: (value: string) => void
  onAmountChange: (value: string) => void
  onReasonTextChange: (value: string) => void
  onAdd: () => void
  onRemove: () => void
  onSet: () => void
}) {
  if (!assets.length) return <p className="admin-inline-muted">Предметов с количеством в каталоге нет.</p>

  return (
    <div className="admin-ownership-form">
      <AdminAssetSelect token={token} assets={assets} value={selectedKey} disabled={disabled} onChange={(assetKey) => onSelectedKeyChange(assetKey)} />
      <input className="ui-input" inputMode="numeric" value={amount} disabled={disabled} onChange={(event) => onAmountChange(event.target.value)} placeholder="Количество" />
      <input className="ui-input" value={reasonText} disabled={disabled} onChange={(event) => onReasonTextChange(event.target.value)} placeholder="Причина операции" />
      <div className="admin-ownership-actions">
        <button className="btn btn-sm" type="button" disabled={disabled} onClick={onSet}>Выставить</button>
        <button className="btn btn-sm danger" type="button" disabled={disabled} onClick={onRemove}>Списать</button>
        <button className="btn btn-sm primary" type="button" disabled={disabled} onClick={onAdd}>Пополнить</button>
      </div>
    </div>
  )
}

function ExpirableControlForm({
  token,
  assets,
  selectedKey,
  durationDays,
  expiresAt,
  reasonText,
  disabled,
  onSelectedKeyChange,
  onDurationDaysChange,
  onExpiresAtChange,
  onReasonTextChange,
  onProlong,
  onSetExpiration,
}: {
  token: string
  assets: AssetResponse[]
  selectedKey: string
  durationDays: string
  expiresAt: string
  reasonText: string
  disabled: boolean
  onSelectedKeyChange: (value: string) => void
  onDurationDaysChange: (value: string) => void
  onExpiresAtChange: (value: string) => void
  onReasonTextChange: (value: string) => void
  onProlong: () => void
  onSetExpiration: () => void
}) {
  if (!assets.length) return <p className="admin-inline-muted">Временных бонусов в каталоге нет.</p>

  return (
    <div className="admin-ownership-form">
      <AdminAssetSelect token={token} assets={assets} value={selectedKey} disabled={disabled} onChange={(assetKey) => onSelectedKeyChange(assetKey)} />
      <input className="ui-input" inputMode="numeric" value={durationDays} disabled={disabled} onChange={(event) => onDurationDaysChange(event.target.value)} placeholder="Дней продления" />
      <input className="ui-input" type="datetime-local" value={expiresAt} disabled={disabled} onChange={(event) => onExpiresAtChange(event.target.value)} />
      <input className="ui-input" value={reasonText} disabled={disabled} onChange={(event) => onReasonTextChange(event.target.value)} placeholder="Причина операции" />
      <div className="admin-ownership-actions">
        <button className="btn btn-sm" type="button" disabled={disabled} onClick={onSetExpiration}>Выставить срок</button>
        <button className="btn btn-sm primary" type="button" disabled={disabled} onClick={onProlong}>Продлить</button>
      </div>
    </div>
  )
}

function AdminInventoryModalList({
  token,
  title,
  items,
  removeLabel,
  emptyText,
  disabled,
}: {
  token: string
  title: string
  items: { key: string; title: string; meta: string; asset: AssetResponse | null; onRemove: () => void }[]
  removeLabel: string
  emptyText: string
  disabled: boolean
}) {
  return (
    <section className="admin-ownership-modal-section">
      <div className="admin-ownership-list-head">
        <h4>{title}</h4>
        <span className="ui-badge ui-badge-neutral">{items.length}</span>
      </div>
      {items.length ? (
        <div className="admin-ownership-modal-list">
          {items.map((item) => (
            <article key={item.key} className="admin-ownership-list-item admin-ownership-list-item--actionable">
              <div className="admin-inventory-item-main">
                <AdminAssetImage
                  token={token}
                  asset={item.asset}
                  className="admin-asset-image-preview--thumb"
                  placeholder="—"
                />
                <span>
                  <strong>{item.title}</strong>
                  <small>{item.meta}</small>
                </span>
              </div>
              <button className="btn btn-sm danger" type="button" disabled={disabled} onClick={item.onRemove}>{removeLabel}</button>
            </article>
          ))}
        </div>
      ) : <p className="admin-inline-muted">{emptyText}</p>}
    </section>
  )
}

function AdminHistoryModalList({ items, emptyText }: { items: { id: number; title: string; meta: string }[]; emptyText: string }) {
  if (!items.length) return <p className="admin-inline-muted">{emptyText}</p>

  return (
    <div className="admin-ownership-modal-list">
      {items.map((item) => (
        <article key={item.id} className="admin-ownership-list-item">
          <strong>{item.title}</strong>
          <small>{item.meta}</small>
        </article>
      ))}
    </div>
  )
}
