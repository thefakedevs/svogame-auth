import { paths } from '../routes/paths'
import { pageTitleForPath } from '../shared/navigation/routes'

const SITE_ORIGIN = 'https://svocraft.xyz'
const DEFAULT_IMAGE = `${SITE_ORIGIN}/landing/hero.jpg`
const DEFAULT_DESCRIPTION =
  'SvoCraft - милитари Minecraft-сервер с ивентовыми событиями, мини-играми, собственным лаунчером и косметикой'

const privatePrefixes = [
  paths.admin,
  paths.profile,
  paths.profileEdit,
  paths.inventory,
  paths.wallet,
  paths.token,
]

function setMeta(selector: string, attribute: 'content' | 'href', value: string) {
  const element = document.querySelector(selector)
  if (element) {
    element.setAttribute(attribute, value)
  }
}

function canonicalUrl(pathname: string) {
  return `${SITE_ORIGIN}${pathname === paths.home ? '/' : pathname}`
}

function isPrivatePath(pathname: string) {
  return privatePrefixes.some((prefix) => pathname === prefix || pathname.startsWith(`${prefix}/`))
}

export function applySeoMeta(pathname: string) {
  const title = pageTitleForPath(pathname)
  const description = DEFAULT_DESCRIPTION
  const canonical = canonicalUrl(pathname)
  const robots = isPrivatePath(pathname) ? 'noindex,nofollow' : 'index,follow'

  document.title = title
  setMeta('meta[name="description"]', 'content', description)
  setMeta('meta[name="robots"]', 'content', robots)
  setMeta('link[rel="canonical"]', 'href', canonical)
  setMeta('meta[property="og:title"]', 'content', title)
  setMeta('meta[property="og:description"]', 'content', description)
  setMeta('meta[property="og:url"]', 'content', canonical)
  setMeta('meta[property="og:image"]', 'content', DEFAULT_IMAGE)
  setMeta('meta[name="twitter:title"]', 'content', title)
  setMeta('meta[name="twitter:description"]', 'content', description)
  setMeta('meta[name="twitter:image"]', 'content', DEFAULT_IMAGE)
}
