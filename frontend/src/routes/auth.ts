import { paths } from './paths'

export function buildAuthUrl(returnUrl?: string | null): string {
  const origin = typeof window === 'undefined' ? 'http://localhost' : window.location.origin
  const url = new URL(paths.auth, origin)

  if (returnUrl && returnUrl !== paths.home) {
    url.searchParams.set('redirectUrl', returnUrl)
  }

  return `${url.pathname}${url.search}`
}

export function currentAppPath(): string {
  if (typeof window === 'undefined') {
    return paths.home
  }

  return `${window.location.pathname}${window.location.search}`
}

export function redirectToAuth(returnUrl?: string | null) {
  window.location.assign(buildAuthUrl(returnUrl))
}
