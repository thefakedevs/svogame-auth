import { paths } from '../../routes/paths'

export function normalizePathname(pathname: string) {
  if (!pathname || pathname === paths.home) {
    return paths.home
  }

  return pathname.endsWith('/') ? pathname.slice(0, -1) : pathname
}

export function pageTitleForPath(pathname: string) {
  if (pathname === paths.home) return 'SvoCraft'
  if (pathname === paths.auth) return 'Авторизация | SvoCraft'
  if (pathname === paths.profileEdit) return 'Настройки профиля | SvoCraft'
  if (pathname === paths.profile) return 'Профиль | SvoCraft'
  if (pathname === paths.leaderboard) return 'Лидерборд | SvoCraft'
  if (pathname === paths.inventory) return 'Инвентарь | SvoCraft'
  if (pathname === paths.shop) return 'Магазин | SvoCraft'
  if (pathname === paths.shopCheckoutReturn) return 'Проверка оплаты | SvoCraft'
  if (pathname === paths.wallet) return 'Кошелек | SvoCraft'
  if (pathname === paths.downloads) return 'Скачать лаунчер | SvoCraft'
  if (pathname === paths.contacts) return 'Контакты | SvoCraft'
  if (pathname === paths.admin) return 'Админ-панель | SvoCraft'
  if (pathname.match(/^\/admin\/users\/[^/]+\/lootboxes\/open-history$/)) {
    return 'Открытые кейсы игрока | Админка | SvoCraft'
  }
  if (pathname.match(/^\/admin\/users\/[^/]+\/littlemice\/[^/]+$/)) {
    return 'Проверка littlemice | Админка | SvoCraft'
  }
  if (pathname.match(/^\/admin\/users\/[^/]+\/littlemice$/)) {
    return 'Проверки littlemice | Админка | SvoCraft'
  }
  if (pathname.startsWith(`${paths.admin}/users/`)) return 'Профиль игрока | Админка | SvoCraft'
  if (pathname.startsWith(`${paths.admin}/squads/`)) return 'Профиль сквада | Админка | SvoCraft'
  if (pathname.startsWith(`${paths.admin}/assets/`)) return 'Ассет | Админка | SvoCraft'
  if (pathname.startsWith(`${paths.admin}/tokens/`) && pathname.endsWith('/audit')) {
    return 'Аудит токена | Админка | SvoCraft'
  }
  if (pathname.startsWith(`${paths.admin}/shop/products/`)) {
    return 'Товар магазина | Админка | SvoCraft'
  }
  if (pathname.startsWith(`${paths.admin}/lootboxes/`)) {
    return 'Лутбокс | Админка | SvoCraft'
  }
  if (pathname === `${paths.admin}/littlemice`) {
    return 'Все проверки littlemice | Админка | SvoCraft'
  }
  if (pathname === paths.token) return 'Завершение авторизации | SvoCraft'
  if (pathname === paths.uiKit) return 'UI Kit | SvoCraft'
  if (pathname === paths.legal) return 'Правовые документы | SvoCraft'
  if (pathname === paths.legalPrivacyPolicy) return 'Политика конфиденциальности | SvoCraft'
  if (pathname === paths.legalPublicOffer) return 'Публичная оферта | SvoCraft'
  if (pathname === paths.legalRefundPolicy) return 'Политика возвратов | SvoCraft'
  if (pathname === paths.legalUserAgreement) return 'Пользовательское соглашение | SvoCraft'
  if (pathname === paths.legalProjectRules) return 'Общие правила проекта | SvoCraft'
  if (pathname === paths.legalCommunityRules) return 'Правила сообщества | SvoCraft'
  if (pathname === paths.legalCtfRules) return 'Правила CTF | SvoCraft'
  if (pathname.startsWith(paths.wiki)) return 'Вики | SvoCraft'
  return 'Авторизация | SvoCraft'
}
