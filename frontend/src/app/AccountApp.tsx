import { useEffect } from 'react'
import { Toaster } from 'react-hot-toast'
import AppHeader from '../components/layout/AppHeader'
import ProfilePage from '../components/profile/ProfilePage'
import HomePage from '../pages/_HomePage'
import UiKitPage from '../pages/_UiKitPage'
import { paths } from '../routes/paths'
import { usePathname } from '../shared/navigation/history'
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

function HomeShell() {
  return (
    <>
      <div className="ui-kit-vhs" aria-hidden />
      <div className="ui-kit-page app-shell">
        <AppHeader />
        <HomePage />
      </div>
    </>
  )
}

export default function AccountApp() {
  const pathname = normalizePathname(usePathname() || paths.home)

  useEffect(() => {
    if (typeof document === 'undefined') {
      return
    }

    document.title = pageTitleForPath(pathname)
  }, [pathname])

  let page = <HomeShell />

  switch (pathname) {
    case paths.auth:
      page = <AuthRoute />
      break
    case paths.profile:
      page = <ProfileShell pageTitle="Профиль" />
      break
    case paths.profileEdit:
      page = <ProfileShell pageTitle="Настройки профиля" defaultTab="settings" />
      break
    case paths.token:
      page = <TokenRoute />
      break
    case paths.uiKit:
      page = <UiKitPage />
      break
    case paths.home:
    default:
      page = <HomeShell />
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
