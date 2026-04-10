import { paths } from '../../routes/paths'

export function normalizePathname(pathname: string) {
  if (!pathname || pathname === paths.home) {
    return paths.home
  }

  return pathname.endsWith('/') ? pathname.slice(0, -1) : pathname
}

export function pageTitleForPath(pathname: string) {
  if (pathname === paths.home) return 'SVOCraft'
  if (pathname === paths.profileEdit) return 'Настройки профиля | SVOCraft'
  if (pathname === paths.profile) return 'Профиль | SVOCraft'
  if (pathname === paths.token) return 'Завершение авторизации | SVOCraft'
  if (pathname === paths.uiKit) return 'UI Kit | SVOCraft'
  return 'Авторизация | SVOCraft'
}
