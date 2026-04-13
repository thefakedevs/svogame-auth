import { useEffect } from 'react'
import { Toaster } from 'react-hot-toast'
import AdminPage from '../components/admin/AdminPage'
import AppHeader from '../components/layout/AppHeader'
import ProfilePage from '../components/profile/ProfilePage'
import ContactsPage from '../pages/_ContactsPage'
import DownloadsPage from '../pages/_DownloadsPage'
import HomePage from '../pages/_HomePage'
import LegalPage, { isLegalPath, legalPageHeading } from '../pages/LegalPage'
import OwnershipPage from '../pages/OwnershipPage'
import UiKitPage from '../pages/_UiKitPage'
import WalletPage from '../pages/WalletPage'
import { paths } from '../routes/paths'
import { buildAnalyticsPath, trackPageView } from '../services/analytics'
import { usePathname, useSearch } from '../shared/navigation/history'
import { normalizePathname, pageTitleForPath } from '../shared/navigation/routes'
import AuthRoute from './AuthRoute'
import TokenRoute from './TokenRoute'

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

function adminRouteForPath(pathname: string): { type: 'home' } | { type: 'user'; userId: string } | { type: 'squad'; squadId: string } | { type: 'tokenAudit'; tokenId: string } | null {
  if (pathname === paths.admin) {
    return { type: 'home' }
  }

  const userMatch = pathname.match(/^\/admin\/users\/([^/]+)$/)
  if (userMatch) {
    return { type: 'user', userId: decodeURIComponent(userMatch[1]) }
  }

  const squadMatch = pathname.match(/^\/admin\/squads\/([^/]+)$/)
  if (squadMatch) {
    return { type: 'squad', squadId: decodeURIComponent(squadMatch[1]) }
  }

  const tokenAuditMatch = pathname.match(/^\/admin\/tokens\/([^/]+)\/audit$/)
  if (tokenAuditMatch) {
    return { type: 'tokenAudit', tokenId: decodeURIComponent(tokenAuditMatch[1]) }
  }

  return null
}

export default function AccountApp() {
  const pathname = normalizePathname(usePathname() || paths.home)
  const search = useSearch()
  const adminRoute = adminRouteForPath(pathname)

  useEffect(() => {
    if (typeof document === 'undefined') {
      return
    }

    document.title = pageTitleForPath(pathname)
  }, [pathname])

  useEffect(() => {
    trackPageView(buildAnalyticsPath(pathname, search), pageTitleForPath(pathname))
  }, [pathname, search])

  let page = <NotFoundShell />

  if (adminRoute) {
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
    case paths.ownership:
      page = <OwnershipShell />
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
      {page}
    </>
  )
}
