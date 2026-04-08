/**
 * Canonical URL paths. Use these for navigation so future refactors
 * (e.g. nesting profile under /cabinet) stay localized.
 */
export const paths = {
  home: '/',
  auth: '/auth',
  profile: '/profile',
  profileEdit: '/profile/edit',
  token: '/token',
  /** Root of the signed-in area; extend with child routes as features land. */
  cabinet: '/cabinet',
  uiKit: '/ui-kit',
} as const

export type AppPath = (typeof paths)[keyof typeof paths]
