import { lazy, Suspense, useEffect, type ReactNode } from 'react'
import { Toaster } from 'react-hot-toast'
import AppHeader from '../components/layout/AppHeader'
import { paths } from '../routes/paths'
import { buildAnalyticsPath, trackPageView } from '../services/analytics'
import { applySeoMeta } from '../services/seo'
import { replaceUrl, usePathname, useSearch } from '../shared/navigation/history'
import { normalizePathname, pageTitleForPath } from '../shared/navigation/routes'

const AdminPage = lazy(() => import('../components/admin/AdminPage'))
const ProfilePage = lazy(() => import('../components/profile/ProfilePage'))
const ContactsPage = lazy(() => import('../pages/_ContactsPage'))
const DownloadsPage = lazy(() => import('../pages/_DownloadsPage'))
const HomePage = lazy(() => import('../pages/_HomePage'))
const LegalPage = lazy(() => import('../pages/LegalPage'))
const LeaderboardPage = lazy(() => import('../pages/LeaderboardPage'))
const OwnershipPage = lazy(() => import('../pages/OwnershipPage'))
const ShopCheckoutReturnPage = lazy(() => import('../pages/ShopCheckoutReturnPage'))
const ShopPage = lazy(() => import('../pages/ShopPage'))
const UiKitPage = lazy(() => import('../pages/_UiKitPage'))
const WalletPage = lazy(() => import('../pages/WalletPage'))
const AuthRoute = lazy(() => import('./AuthRoute'))
const TokenRoute = lazy(() => import('./TokenRoute'))

const legalPaths = new Set<string>([
  paths.legal,
  paths.legalPrivacyPolicy,
  paths.legalPublicOffer,
  paths.legalRefundPolicy,
  paths.legalUserAgreement,
  paths.legalProjectRules,
  paths.legalCommunityRules,
  paths.legalCtfRules,
])

const legalHeadings = new Map<string, string>([
  [paths.legal, 'Правовые документы'],
  [paths.legalPrivacyPolicy, 'Политика конфиденциальности'],
  [paths.legalPublicOffer, 'Публичная оферта'],
  [paths.legalRefundPolicy, 'Политика возвратов'],
  [paths.legalUserAgreement, 'Пользовательское соглашение'],
  [paths.legalProjectRules, 'Общие правила проекта'],
  [paths.legalCommunityRules, 'Правила сообщества'],
  [paths.legalCtfRules, 'Правила режима CTF'],
])

function isLegalPath(pathname: string) {
  return legalPaths.has(pathname)
}

function legalPageHeading(pathname: string) {
  return legalHeadings.get(pathname) ?? 'Правовые документы'
}

function RoutePending() {
  const shouldRenderRoutePending = false
  if (!shouldRenderRoutePending) return null

  return (
    <div className="page">
      <div className="route-pending" role="status" aria-label="Загрузка страницы">
        <span className="ui-spinner" aria-hidden>
          <span className="ui-spinner-track">
            <span className="ui-spinner-orb" />
            <span className="ui-spinner-orb" />
            <span className="ui-spinner-orb" />
            <span className="ui-spinner-orb" />
          </span>
        </span>
      </div>
    </div>
  )
}

function RouteSuspense({ children }: { children: ReactNode }) {
  return <Suspense fallback={<RoutePending />}>{children}</Suspense>
}

function authReferralRedirectUrl(pathname: string, search: string) {
  if (pathname !== paths.home || !search) return null

  const params = new URLSearchParams(search)
  const referralCode = params.get('ref')
  if (!referralCode) return null

  return `${paths.auth}?ref=${encodeURIComponent(referralCode)}`
}

function ProfileShell({ pageTitle, defaultTab }: { pageTitle: string; defaultTab?: 'settings' }) {
  return (
    <>
      <div className="ui-kit-vhs" aria-hidden />
      <div className="ui-kit-page app-shell">
        <AppHeader pageTitle={pageTitle} />
        <ProfilePage defaultTab={defaultTab} />
      </div>
    </>
  )
}

function OwnershipShell() {
  return (
    <>
      <div className="ui-kit-vhs" aria-hidden />
      <div className="ui-kit-page app-shell">
        <AppHeader pageTitle="Инвентарь" />
        <OwnershipPage />
      </div>
    </>
  )
}

function LeaderboardShell() {
  return (
    <>
      <div className="ui-kit-vhs" aria-hidden />
      <div className="ui-kit-page app-shell">
        <AppHeader pageTitle="Лидерборд" />
        <LeaderboardPage />
      </div>
    </>
  )
}

function ShopShell() {
  return (
    <>
      <div className="ui-kit-vhs" aria-hidden />
      <div className="ui-kit-page app-shell">
        <AppHeader pageTitle="Магазин" />
        <ShopPage />
      </div>
    </>
  )
}

function ShopCheckoutReturnShell() {
  return (
    <>
      <div className="ui-kit-vhs" aria-hidden />
      <div className="ui-kit-page app-shell">
        <AppHeader pageTitle="Проверка оплаты" />
        <ShopCheckoutReturnPage />
      </div>
    </>
  )
}

function WalletShell() {
  return (
    <>
      <div className="ui-kit-vhs" aria-hidden />
      <div className="ui-kit-page app-shell">
        <AppHeader pageTitle="Кошелек" />
        <WalletPage />
      </div>
    </>
  )
}

function HomeShell() {
  return (
    <>
      <div className="ui-kit-vhs" aria-hidden />
      <div className="ui-kit-page app-shell landing-shell">
        <AppHeader />
        <HomePage />
      </div>
    </>
  )
}

function AdminShell() {
  return (
    <>
      <div className="ui-kit-vhs" aria-hidden />
      <div className="ui-kit-page app-shell">
        <AppHeader pageTitle="Админка" />
        <AdminPage />
      </div>
    </>
  )
}

function LegalShell({ pathname }: { pathname: string }) {
  return (
    <>
      <div className="ui-kit-vhs" aria-hidden />
      <div className="ui-kit-page app-shell">
        <AppHeader pageTitle={legalPageHeading(pathname)} />
        <LegalPage pathname={pathname} />
      </div>
    </>
  )
}

function ContactsShell() {
  return (
    <>
      <div className="ui-kit-vhs" aria-hidden />
      <div className="ui-kit-page app-shell">
        <AppHeader pageTitle="Контакты" />
        <ContactsPage />
      </div>
    </>
  )
}

function DownloadsShell() {
  return (
    <>
      <div className="ui-kit-vhs" aria-hidden />
      <div className="ui-kit-page app-shell">
        <AppHeader pageTitle="Скачать лаунчер" />
        <DownloadsPage />
      </div>
    </>
  )
}

function NotFoundShell() {
  return (
    <>
      <div className="ui-kit-vhs" aria-hidden />
      <div className="ui-kit-page app-shell">
        <AppHeader pageTitle="Страница не найдена" />
        <div className="page">
          <section className="card">
            <h1 className="card-title">Страница не найдена</h1>
            <p className="card-text">Проверьте адрес или вернитесь на главную страницу.</p>
          </section>
        </div>
      </div>
    </>
  )
}

function adminRouteForPath(
  pathname: string,
):
  | { type: 'home' }
  | { type: 'user'; userId: string }
  | { type: 'squad'; squadId: string }
  | { type: 'tokenAudit'; tokenId: string }
  | { type: 'asset'; assetId: string }
  | { type: 'shopProduct'; productId: string }
  | { type: 'lootbox'; lootboxId: string }
  | { type: 'littlemice' }
  | { type: 'userLootboxHistory'; userId: string }
  | { type: 'userLittlemice'; userId: string; checkId?: string }
  | null {
  if (pathname === paths.admin) {
    return { type: 'home' }
  }

  const userMatch = pathname.match(/^\/admin\/users\/([^/]+)$/)
  if (userMatch) {
    return { type: 'user', userId: decodeURIComponent(userMatch[1]) }
  }

  const userLootboxHistoryMatch = pathname.match(/^\/admin\/users\/([^/]+)\/lootboxes\/open-history$/)
  if (userLootboxHistoryMatch) {
    return { type: 'userLootboxHistory', userId: decodeURIComponent(userLootboxHistoryMatch[1]) }
  }

  const userLittlemiceCheckMatch = pathname.match(/^\/admin\/users\/([^/]+)\/littlemice\/([^/]+)$/)
  if (userLittlemiceCheckMatch) {
    return {
      type: 'userLittlemice',
      userId: decodeURIComponent(userLittlemiceCheckMatch[1]),
      checkId: decodeURIComponent(userLittlemiceCheckMatch[2]),
    }
  }

  const userLittlemiceMatch = pathname.match(/^\/admin\/users\/([^/]+)\/littlemice$/)
  if (userLittlemiceMatch) {
    return { type: 'userLittlemice', userId: decodeURIComponent(userLittlemiceMatch[1]) }
  }

  const squadMatch = pathname.match(/^\/admin\/squads\/([^/]+)$/)
  if (squadMatch) {
    return { type: 'squad', squadId: decodeURIComponent(squadMatch[1]) }
  }

  const tokenAuditMatch = pathname.match(/^\/admin\/tokens\/([^/]+)\/audit$/)
  if (tokenAuditMatch) {
    return { type: 'tokenAudit', tokenId: decodeURIComponent(tokenAuditMatch[1]) }
  }

  const assetMatch = pathname.match(/^\/admin\/assets\/([^/]+)$/)
  if (assetMatch) {
    return { type: 'asset', assetId: decodeURIComponent(assetMatch[1]) }
  }

  const shopProductMatch = pathname.match(/^\/admin\/shop\/products\/([^/]+)$/)
  if (shopProductMatch) {
    return { type: 'shopProduct', productId: decodeURIComponent(shopProductMatch[1]) }
  }

  const lootboxMatch = pathname.match(/^\/admin\/lootboxes\/([^/]+)$/)
  if (lootboxMatch) {
    return { type: 'lootbox', lootboxId: decodeURIComponent(lootboxMatch[1]) }
  }

  if (pathname === `${paths.admin}/littlemice`) {
    return { type: 'littlemice' }
  }

  return null
}

export default function AccountApp() {
  const pathname = normalizePathname(usePathname() || paths.home)
  const search = useSearch()
  const adminRoute = adminRouteForPath(pathname)
  const referralRedirectUrl = authReferralRedirectUrl(pathname, search)

  useEffect(() => {
    if (!referralRedirectUrl) return
    replaceUrl(referralRedirectUrl)
  }, [referralRedirectUrl])

  useEffect(() => {
    if (typeof document === 'undefined') {
      return
    }

    applySeoMeta(pathname)
  }, [pathname])

  useEffect(() => {
    trackPageView(buildAnalyticsPath(pathname, search), pageTitleForPath(pathname))
  }, [pathname, search])

  let page = referralRedirectUrl ? null : <NotFoundShell />

  if (referralRedirectUrl) {
    page = null
  } else if (adminRoute) {
    page = <AdminShell />
  } else if (isLegalPath(pathname)) {
    page = <LegalShell pathname={pathname} />
  } else switch (pathname) {
    case paths.auth:
      page = <AuthRoute />
      break
    case paths.profile:
      page = <ProfileShell pageTitle="Профиль" />
      break
    case paths.profileEdit:
      page = <ProfileShell pageTitle="Настройки профиля" defaultTab="settings" />
      break
    case paths.leaderboard:
      page = <LeaderboardShell />
      break
    case paths.inventory:
      page = <OwnershipShell />
      break
    case paths.shop:
      page = <ShopShell />
      break
    case paths.shopCheckoutReturn:
      page = <ShopCheckoutReturnShell />
      break
    case paths.wallet:
      page = <WalletShell />
      break
    case paths.contacts:
      page = <ContactsShell />
      break
    case paths.downloads:
      page = <DownloadsShell />
      break
    case paths.token:
      page = <TokenRoute />
      break
    case paths.uiKit:
      page = <UiKitPage />
      break
    case paths.home:
      page = <HomeShell />
      break
    default:
      page = <NotFoundShell />
      break
  }

  return (
    <>
      <Toaster
        position="top-right"
        containerStyle={{
          zIndex: 10000,
        }}
        toastOptions={{
          duration: 3000,
          style: {
            background: '#19191c',
            color: '#fff',
            border: '1px solid rgba(255,255,255,.08)',
            borderRadius: '0',
          },
        }}
      />
      <RouteSuspense>{page}</RouteSuspense>
    </>
  )
}
