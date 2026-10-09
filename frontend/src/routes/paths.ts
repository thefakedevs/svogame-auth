/**
 * Canonical URL paths for the frontend.
 */
import { withBasePath } from '../shared/navigation/basePath'

export const paths = {
  home: withBasePath('/'),
  auth: withBasePath('/auth'),
  profile: withBasePath('/profile'),
  profileEdit: withBasePath('/profile/edit'),
  leaderboard: withBasePath('/leaderboard'),
  inventory: withBasePath('/inventory'),
  shop: withBasePath('/shop'),
  shopCheckoutReturn: withBasePath('/shop/checkout/return'),
  wallet: withBasePath('/wallet'),
  downloads: withBasePath('/downloads'),
  contacts: withBasePath('/contacts'),
  admin: withBasePath('/admin'),
  token: withBasePath('/token'),
  uiKit: withBasePath('/ui-kit'),
  legal: withBasePath('/legal'),
  legalPrivacyPolicy: withBasePath('/legal/privacy_policy'),
  legalPublicOffer: withBasePath('/legal/public_offer'),
  legalRefundPolicy: withBasePath('/legal/refund_policy'),
  legalUserAgreement: withBasePath('/legal/user_agreement'),
  legalProjectRules: withBasePath('/legal/project_rules'),
  legalCommunityRules: withBasePath('/legal/community_rules'),
  legalCtfRules: withBasePath('/legal/ctf_rules'),
  wiki: withBasePath('/wiki'),
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

export function adminAssetPath(assetId: string) {
  return `${paths.admin}/assets/${encodeURIComponent(assetId)}`
}

export function adminLootboxPath(lootboxId: string) {
  return `${paths.admin}/lootboxes/${encodeURIComponent(lootboxId)}`
}

export function adminLittlemicePath() {
  return `${paths.admin}/littlemice`
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
