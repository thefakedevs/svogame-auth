/**
 * Canonical URL paths for the frontend.
 */
export const paths = {
  home: '/',
  auth: '/auth',
  profile: '/profile',
  profileEdit: '/profile/edit',
  inventory: '/inventory',
  wallet: '/wallet',
  downloads: '/downloads',
  contacts: '/contacts',
  admin: '/admin',
  token: '/token',
  uiKit: '/ui-kit',
  legal: '/legal',
  legalPrivacyPolicy: '/legal/privacy_policy',
  legalPublicOffer: '/legal/public_offer',
  legalRefundPolicy: '/legal/refund_policy',
  legalUserAgreement: '/legal/user_agreement',
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
