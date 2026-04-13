type UmamiPayload = Record<string, unknown>
type UmamiPayloadBuilder = (props: UmamiPayload) => UmamiPayload

type UmamiTracker = {
  track: {
    (): void
    (payload: UmamiPayload | UmamiPayloadBuilder): void
    (eventName: string, data?: UmamiPayload): void
  }
}

type TrackerCallback = (tracker: UmamiTracker) => void

declare global {
  interface Window {
    umami?: UmamiTracker
    __svocraftAnalyticsInitialized?: boolean
    __svocraftAnalyticsInteractionsInstalled?: boolean
  }
}

const DEFAULT_UMAMI_SCRIPT_URL = 'https://analytics.artembay.ru/script.js'
const DEFAULT_UMAMI_WEBSITE_ID = '0c965545-fd3c-4485-9821-5267012848a5'
const UMAMI_SCRIPT_ID = 'svocraft-umami-script'
const MAX_QUEUED_CALLS = 50
const MAX_EVENT_NAME_LENGTH = 50
const MAX_EVENT_DATA_STRING_LENGTH = 500
const SAFE_QUERY_KEYS = new Set(['tab'])

const pendingCalls: TrackerCallback[] = []

function envValue(key: keyof ImportMetaEnv): string {
  return import.meta.env[key]?.trim() ?? ''
}

function analyticsEnabled() {
  return envValue('VITE_UMAMI_ENABLED') !== 'false'
}

function umamiWebsiteId() {
  return envValue('VITE_UMAMI_WEBSITE_ID') || DEFAULT_UMAMI_WEBSITE_ID
}

function umamiScriptUrl() {
  return envValue('VITE_UMAMI_SCRIPT_URL') || DEFAULT_UMAMI_SCRIPT_URL
}

function runWithTracker(callback: TrackerCallback) {
  if (!analyticsEnabled() || typeof window === 'undefined') return

  if (window.umami?.track) {
    callback(window.umami)
    return
  }

  if (pendingCalls.length < MAX_QUEUED_CALLS) {
    pendingCalls.push(callback)
  }
}

function flushPendingCalls() {
  if (typeof window === 'undefined' || !window.umami?.track) return

  const calls = pendingCalls.splice(0, pendingCalls.length)
  for (const callback of calls) {
    callback(window.umami)
  }
}

function normalizeEventName(name: string) {
  const normalized = name.trim().replace(/\s+/g, '_').slice(0, MAX_EVENT_NAME_LENGTH)
  return normalized || 'interaction'
}

function normalizeEventValue(value: unknown): string | number | boolean | null {
  if (typeof value === 'number' || typeof value === 'boolean') return value
  if (value == null) return null
  return String(value).slice(0, MAX_EVENT_DATA_STRING_LENGTH)
}

function compactEventData(data: UmamiPayload) {
  const entries = Object.entries(data)
    .filter(([, value]) => value !== undefined && value !== '')
    .slice(0, 50)
    .map(([key, value]) => [key, normalizeEventValue(value)])

  return Object.fromEntries(entries)
}

function normalizeDynamicPathname(pathname: string) {
  return pathname
    .replace(/^\/admin\/users\/[^/]+$/, '/admin/users/:userId')
    .replace(/^\/admin\/squads\/[^/]+$/, '/admin/squads/:squadId')
    .replace(/^\/admin\/tokens\/[^/]+\/audit$/, '/admin/tokens/:tokenId/audit')
}

export function buildAnalyticsPath(pathname: string, search = '') {
  const normalizedPathname = normalizeDynamicPathname(pathname || '/')
  const params = new URLSearchParams(search)
  const safeParams = new URLSearchParams()

  for (const [key, value] of params.entries()) {
    if (SAFE_QUERY_KEYS.has(key)) {
      safeParams.set(key, value)
    }
  }

  const safeSearch = safeParams.toString()
  return safeSearch ? `${normalizedPathname}?${safeSearch}` : normalizedPathname
}

function currentAnalyticsPath() {
  if (typeof window === 'undefined') return '/'
  return buildAnalyticsPath(window.location.pathname, window.location.search)
}

export function initAnalytics() {
  if (!analyticsEnabled() || typeof document === 'undefined') return
  if (window.__svocraftAnalyticsInitialized) return

  const websiteId = umamiWebsiteId()
  const scriptUrl = umamiScriptUrl()

  if (!websiteId || !scriptUrl) return

  window.__svocraftAnalyticsInitialized = true

  const script = document.createElement('script')
  script.id = UMAMI_SCRIPT_ID
  script.async = true
  script.defer = true
  script.src = scriptUrl
  script.dataset.websiteId = websiteId
  script.dataset.autoTrack = 'false'
  script.dataset.doNotTrack = envValue('VITE_UMAMI_DO_NOT_TRACK') === 'false' ? 'false' : 'true'

  const hostUrl = envValue('VITE_UMAMI_HOST_URL')
  const domains = envValue('VITE_UMAMI_DOMAINS')
  const tag = envValue('VITE_UMAMI_TAG')

  if (hostUrl) script.dataset.hostUrl = hostUrl
  if (domains) script.dataset.domains = domains
  if (tag) script.dataset.tag = tag

  script.addEventListener('load', () => {
    flushPendingCalls()
    window.setTimeout(flushPendingCalls, 250)
  })
  script.addEventListener('error', () => {
    window.__svocraftAnalyticsInitialized = false
  })

  document.head.appendChild(script)
}

export function trackPageView(path: string, title: string) {
  runWithTracker((tracker) => {
    tracker.track((props) => ({
      ...props,
      url: path,
      title,
    }))
  })
}

export function trackEvent(eventName: string, data: UmamiPayload = {}) {
  runWithTracker((tracker) => {
    tracker.track(normalizeEventName(eventName), compactEventData(data))
  })
}

function dataAttributes(element: Element, prefix: string) {
  const values: UmamiPayload = {}

  for (const attribute of Array.from(element.attributes)) {
    if (!attribute.name.startsWith(prefix)) continue

    const key = attribute.name.slice(prefix.length).replace(/-/g, '_')
    if (key) values[key] = attribute.value
  }

  return values
}

function findTrackableElement(target: EventTarget | null) {
  if (!(target instanceof Element)) return null

  return target.closest(
    [
      '[data-analytics-event]',
      '[data-umami-event]',
      'a[href]',
      'button',
      'input[type="button"]',
      'input[type="submit"]',
      'input[type="reset"]',
      '[role="button"]',
      'summary',
    ].join(','),
  )
}

function elementKind(element: Element) {
  const tagName = element.tagName.toLowerCase()
  if (element instanceof HTMLAnchorElement) return 'link'
  if (element instanceof HTMLButtonElement) return 'button'
  if (element instanceof HTMLInputElement) return element.type || 'input'
  if (element.getAttribute('role') === 'button') return 'button'
  return tagName
}

function clickEventName(element: Element) {
  const explicitName = element.getAttribute('data-analytics-event') || element.getAttribute('data-umami-event')
  if (explicitName) return explicitName
  if (element instanceof HTMLAnchorElement) return 'link_click'
  return 'button_click'
}

function linkData(element: Element) {
  if (!(element instanceof HTMLAnchorElement)) return {}

  const url = new URL(element.href, window.location.href)
  const outbound = url.origin !== window.location.origin

  return {
    href_kind: outbound ? 'external' : 'internal',
    href_host: outbound ? url.hostname : undefined,
    download: element.hasAttribute('download') ? 'true' : undefined,
  }
}

function interactionData(element: Element, interaction: 'click' | 'submit') {
  return {
    interaction,
    element: elementKind(element),
    path: currentAnalyticsPath(),
    cta: element.getAttribute('data-cta') ?? undefined,
    ...linkData(element),
    ...dataAttributes(element, 'data-analytics-event-'),
    ...dataAttributes(element, 'data-umami-event-'),
  }
}

function onDocumentClick(event: MouseEvent) {
  const element = findTrackableElement(event.target)
  if (!element) return

  trackEvent(clickEventName(element), interactionData(element, 'click'))
}

function onDocumentSubmit(event: SubmitEvent) {
  if (!(event.target instanceof HTMLFormElement)) return

  const form = event.target
  const eventName = form.getAttribute('data-analytics-event') || form.getAttribute('data-umami-event') || 'form_submit'
  trackEvent(eventName, interactionData(form, 'submit'))
}

export function installAnalyticsInteractionTracking() {
  if (!analyticsEnabled() || typeof document === 'undefined') return
  if (window.__svocraftAnalyticsInteractionsInstalled) return

  window.__svocraftAnalyticsInteractionsInstalled = true
  document.addEventListener('click', onDocumentClick, true)
  document.addEventListener('submit', onDocumentSubmit, true)
}
