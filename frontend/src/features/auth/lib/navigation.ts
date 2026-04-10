import { paths } from '../../../routes/paths'
import { currentAppPath, navigateTo } from '../../../shared/navigation/history'

export function buildAuthUrl(returnUrl?: string | null): string {
  const origin = typeof window === 'undefined' ? 'http://localhost' : window.location.origin
  const url = new URL(paths.auth, origin)

  if (returnUrl && returnUrl !== paths.home) {
    url.searchParams.set('redirectUrl', returnUrl)
  }

  return `${url.pathname}${url.search}`
}

export function redirectToAuth(returnUrl?: string | null) {
  navigateTo(buildAuthUrl(returnUrl), { replace: true })
}

export function getCurrentAppPath() {
  return currentAppPath()
}

export function normalizeLoopbackHost() {
  if (typeof window === 'undefined' || window.location.hostname !== '127.0.0.1') {
    return false
  }

  const nextUrl = new URL(window.location.href)
  nextUrl.hostname = 'localhost'
  window.location.replace(nextUrl.toString())
  return true
}
