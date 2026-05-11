/**
 * Canonical URL paths for the frontend.
 */
export const paths = {
  home: '/',
  auth: '/auth',
  profile: '/profile',
  profileEdit: '/profile/edit',
  inventory: '/inventory',
  shop: '/shop',
  shopCheckoutReturn: '/shop/checkout/return',
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
  legalProjectRules: '/legal/project_rules',
  legalCommunityRules: '/legal/community_rules',
  legalCtfRules: '/legal/ctf_rules',
  wiki: '/wiki',
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

export function adminShopProductPath(productId: string) {
  return `${paths.admin}/shop/products/${encodeURIComponent(productId)}`
}

export function adminLootboxPath(lootboxId: string) {
  return `${paths.admin}/lootboxes/${encodeURIComponent(lootboxId)}`
}

export function adminUserLootboxHistoryPath(userId: string) {
  return `${paths.admin}/users/${encodeURIComponent(userId)}/lootboxes/open-history`
}

export function adminUserLittlemicePath(userId: string) {
  return `${paths.admin}/users/${encodeURIComponent(userId)}/littlemice`
}

export function adminUserLittlemiceCheckPath(userId: string, checkId: string) {
  return `${adminUserLittlemicePath(userId)}/${encodeURIComponent(checkId)}`
}
