/**
 * Canonical URL paths for the frontend.
 */
export const paths = {
  home: '/',
  auth: '/auth',
  profile: '/profile',
  profileEdit: '/profile/edit',
  token: '/token',
  uiKit: '/ui-kit',
} as const

export type AppPath = (typeof paths)[keyof typeof paths]
