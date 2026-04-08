import { useEffect, useRef, useState } from 'react'
import {
  getMyInventory,
  getMyWallet,
  getPublicAssets,
  type WalletBalanceResponse,
} from '../services/profileHubApi'
import { tokenManager } from '../services/tokenManager'
import { paths } from '../routes/paths'
import { useAuthStore } from '../store/authStore'

type Props = {
  pathname: string
}

function formatWalletLabel(wallet: WalletBalanceResponse[]) {
  if (!wallet.length) return 'Кошелек пуст'
  if (wallet.length === 1) return `${wallet[0].balance} на балансе`
  return `${wallet.length} баланса`
}

export default function CabinetHeader({ pathname }: Props) {
  const authHydrated = useAuthStore((store) => store.hydrated)
  const authUser = useAuthStore((store) => store.user)
  const authToken = useAuthStore((store) => store.token)
  const setAuthUser = useAuthStore((store) => store.setUser)
  const setAuthToken = useAuthStore((store) => store.setToken)

  const [isMenuOpen, setIsMenuOpen] = useState(false)
  const [wallet, setWallet] = useState<WalletBalanceResponse[]>([])
  const [subscriptionCount, setSubscriptionCount] = useState(0)
  const menuRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    const onPointerDown = (event: PointerEvent) => {
      if (!menuRef.current?.contains(event.target as Node)) {
        setIsMenuOpen(false)
      }
    }
    window.addEventListener('pointerdown', onPointerDown)
    return () => window.removeEventListener('pointerdown', onPointerDown)
  }, [])

  useEffect(() => {
    if (!authHydrated || !authToken) return
    let cancelled = false

    Promise.all([getMyWallet(authToken), getMyInventory(authToken), getPublicAssets()])
      .then(([walletData, inventory, assets]) => {
        if (cancelled) return
        const assetMap = new Map(assets.map((asset) => [asset.key, asset.assetKind]))
        const items = [...inventory.stackables, ...inventory.entitlements, ...inventory.expirables]
        const activeSubscriptions = items.filter((item) => assetMap.get(item.assetKey) === 'subscription').length
        setWallet(walletData)
        setSubscriptionCount(activeSubscriptions)
      })
      .catch(() => {
        if (cancelled) return
        setWallet([])
        setSubscriptionCount(0)
      })

    return () => {
      cancelled = true
    }
  }, [authHydrated, authToken])

  const avatarFallback = authUser?.username ? authUser.username.slice(0, 2).toUpperCase() : 'SV'
  const visibleWallet = authToken ? wallet : []
  const visibleSubscriptionCount = authToken ? subscriptionCount : 0
  const isReady = authHydrated

  const onLogout = () => {
    tokenManager.clearToken()
    setAuthToken(null)
    setAuthUser(null)
    window.location.assign(paths.auth)
  }

  return (
    <header className="cabinet-layout__header">
      <a href={paths.home} className="cabinet-layout__back">← На главную</a>
      <div className="cabinet-layout__controls">
        <nav className="cabinet-layout__nav" aria-label="Кабинет">
          <a href={paths.cabinet} aria-current={pathname === paths.cabinet ? 'page' : undefined}>Обзор</a>
          <a href={paths.profile} aria-current={pathname === paths.profile ? 'page' : undefined}>Профиль</a>
        </nav>

        <div className="cabinet-layout__account" ref={menuRef}>
          <button
            className={`cabinet-layout__account-trigger${!isReady ? ' is-placeholder' : ''}`}
            type="button"
            onClick={() => setIsMenuOpen((open) => !open)}
            aria-expanded={isMenuOpen}
            disabled={!isReady}
          >
            {isReady ? (
              authUser?.avatarUrl ? <img src={authUser.avatarUrl} alt={authUser.username} className="cabinet-layout__avatar" /> : <span className="cabinet-layout__avatar cabinet-layout__avatar--fallback">{avatarFallback}</span>
            ) : (
              <span className="cabinet-layout__avatar cabinet-layout__avatar--placeholder" aria-hidden />
            )}
            <span className={`cabinet-layout__account-name${!isReady ? ' cabinet-layout__account-name--placeholder' : ''}`}>
              {isReady ? authUser?.username ?? 'Игрок' : ''}
            </span>
            <span className="cabinet-layout__account-caret" aria-hidden>{isReady ? '▾' : ''}</span>
          </button>

          {isReady && isMenuOpen ? <div className="cabinet-layout__menu"><div className="cabinet-layout__menu-head">{authUser?.avatarUrl ? <img src={authUser.avatarUrl} alt={authUser.username} className="cabinet-layout__menu-avatar" /> : <span className="cabinet-layout__menu-avatar cabinet-layout__avatar--fallback">{avatarFallback}</span>}<div><strong>{authUser?.username ?? 'Игрок'}</strong><div className="cabinet-layout__menu-meta">Discord подключен</div></div></div><div className="cabinet-layout__menu-pills"><span className="ui-badge ui-badge-neutral">{formatWalletLabel(visibleWallet)}</span><span className={`ui-badge ${visibleSubscriptionCount > 0 ? 'ui-badge-secondary' : 'ui-badge-neutral'}`}>{visibleSubscriptionCount > 0 ? 'Подписка активна' : 'Без подписки'}</span></div><div className="cabinet-layout__menu-section"><a href={paths.profile}>Профиль</a><a href={`${paths.profile}?tab=inventory`}>Инвентарь</a><a href={`${paths.profile}?tab=squads`}>Сквад</a><a href={`${paths.profile}?tab=settings`}>Настройки</a></div><div className="cabinet-layout__menu-section cabinet-layout__menu-section--danger"><button type="button" onClick={onLogout}>Выйти</button></div></div> : null}
        </div>
      </div>
    </header>
  )
}
