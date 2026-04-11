import { paths } from '../../routes/paths'

export function normalizePathname(pathname: string) {
  if (!pathname || pathname === paths.home) {
    return paths.home
  }

  return pathname.endsWith('/') ? pathname.slice(0, -1) : pathname
}

export function pageTitleForPath(pathname: string) {
  if (pathname === paths.home) return 'SVOCraft'
  if (pathname === paths.auth) return 'Авторизация | SVOCraft'
  if (pathname === paths.profileEdit) return 'Настройки профиля | SVOCraft'
  if (pathname === paths.profile) return 'Профиль | SVOCraft'
  if (pathname === paths.downloads) return 'Скачать лаунчер | SVOCraft'
  if (pathname === paths.contacts) return 'Контакты | SVOCraft'
  if (pathname === paths.admin) return 'Админ-панель | SVOCraft'
  if (pathname.startsWith(`${paths.admin}/users/`)) return 'Профиль игрока | Админка | SVOCraft'
  if (pathname.startsWith(`${paths.admin}/squads/`)) return 'Профиль сквада | Админка | SVOCraft'
  if (pathname.startsWith(`${paths.admin}/tokens/`) && pathname.endsWith('/audit')) {
    return 'Аудит токена | Админка | SVOCraft'
  }
  if (pathname === paths.token) return 'Завершение авторизации | SVOCraft'
  if (pathname === paths.uiKit) return 'UI Kit | SVOCraft'
  if (pathname === paths.legal) return 'Правовые документы | SVOCraft'
  if (pathname === paths.legalPrivacyPolicy) return 'Политика конфиденциальности | SVOCraft'
  if (pathname === paths.legalPublicOffer) return 'Публичная оферта | SVOCraft'
  if (pathname === paths.legalRefundPolicy) return 'Политика возвратов | SVOCraft'
  if (pathname === paths.legalUserAgreement) return 'Пользовательское соглашение | SVOCraft'
  return 'Авторизация | SVOCraft'
}
