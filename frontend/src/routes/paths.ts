/**
 * Canonical URL paths for the frontend.
 */
export const paths = {
  home: '/',
  auth: '/auth',
  profile: '/profile',
  profileEdit: '/profile/edit',
  admin: '/admin',
  token: '/token',
  uiKit: '/ui-kit',
} as const

export type AppPath = (typeof paths)[keyof typeof paths]

export function adminUserPath(userId: string) {
  return `${paths.admin}/users/${encodeURIComponent(userId)}`
}

export function adminSquadPath(squadId: string) {
  return `${paths.admin}/squads/${encodeURIComponent(squadId)}`
}

export function adminTokenAuditPath(tokenId: string) {
  return `${paths.admin}/tokens/${encodeURIComponent(tokenId)}/audit`
}
