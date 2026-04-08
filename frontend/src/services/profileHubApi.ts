import { ApiError } from './userApi'

export interface AssetResponse {
  id: string
  key: string
  displayName: string
  description: string | null
  assetKind: string
  ownershipModel: string
  isCurrency: boolean
  isPublic: boolean
  isUserPurchasable: boolean
  isActive: boolean
  metadata: unknown
  createdAt: string
  updatedAt: string
}

export interface AssetListResponse {
  items: AssetResponse[]
  total: number
  page: number
  perPage: number
  totalPages: number
}

export interface StackableResponse {
  assetDefinitionId: string
  assetKey: string
  amount: number
  updatedAt: string
}

export interface EntitlementResponse {
  assetDefinitionId: string
  assetKey: string
  grantedAt: string
  updatedAt: string
}

export interface ExpirableResponse {
  assetDefinitionId: string
  assetKey: string
  expiresAt: string
  grantedAt: string
  updatedAt: string
  isActive: boolean
  lastExtendedAt: string | null
}

export interface InventoryResponse {
  userId: string
  stackables: StackableResponse[]
  entitlements: EntitlementResponse[]
  expirables: ExpirableResponse[]
}

export interface UserRestrictionResponse {
  key: string
  createdAt: string
  reason: string | null
}

export interface RestrictionMetaResponse {
  key: string
  locale: {
    en: {
      title: string
      description: string
    }
  }
}

export interface SquadResponse {
  id: string
  name: string
  leaderUserId: string
  memberCount: number
  maxMembers: number
  imageUrl: string | null
  isRestricted: boolean
  restrictionReason: string | null
  createdAt: string
  updatedAt: string
}

export interface SquadMemberResponse {
  id: string
  username: string
  avatarUrl: string | null
  isLeader: boolean
}

export interface SquadInviteResponse {
  id: string
  squadId: string
  squadName: string
  inviterUserId: string
  invitedUserId: string
  expiresAt: string
  createdAt: string
}

export interface WalletBalanceResponse {
  userId: string
  currencyAssetDefinitionId: string
  currencyKey: string
  balance: number
  updatedAt: string
}

export interface SquadConfigResponse {
  maxMembers: number
  inviteTtlHours: number
  nameMinChars: number
  nameMaxChars: number
  nameRegex: string
  imageMaxBytes: number
  imageMaxWidth: number
  imageMaxHeight: number
}

async function readError(response: Response): Promise<string> {
  try {
    const body = await response.json()
    if (body?.error) return body.error
    if (body?.message) return body.message
  } catch {
    // ignore parse errors
  }

  return response.statusText
}

async function requestJson<T>(input: string, init?: RequestInit): Promise<T> {
  const response = await fetch(input, init)
  if (!response.ok) {
    throw new ApiError(response.status, response.statusText, await readError(response))
  }
  return response.json() as Promise<T>
}

function authHeaders(token: string): HeadersInit {
  return {
    Authorization: `Bearer ${token}`,
    'Content-Type': 'application/json',
  }
}

export async function getPublicAssets(): Promise<AssetResponse[]> {
  const firstPage = await requestJson<AssetListResponse>('/api/assets?perPage=100&page=1')

  if (firstPage.totalPages <= 1) {
    return firstPage.items
  }

  const pages = await Promise.all(
    Array.from({ length: firstPage.totalPages - 1 }, (_item, index) =>
      requestJson<AssetListResponse>(`/api/assets?perPage=100&page=${index + 2}`),
    ),
  )

  return firstPage.items.concat(...pages.map((page) => page.items))
}

export function getMyInventory(token: string): Promise<InventoryResponse> {
  return requestJson<InventoryResponse>('/api/user/me/inventory', {
    headers: authHeaders(token),
  })
}

export function getMyRestrictions(token: string): Promise<UserRestrictionResponse[]> {
  return requestJson<UserRestrictionResponse[]>('/api/user/me/restrictions', {
    headers: authHeaders(token),
  })
}

export async function getMySquad(token: string): Promise<SquadResponse | null> {
  const response = await fetch('/api/user/me/squad', {
    headers: authHeaders(token),
  })

  if (response.status === 404) {
    return null
  }

  if (!response.ok) {
    throw new ApiError(response.status, response.statusText, await readError(response))
  }

  return response.json() as Promise<SquadResponse>
}

export function getMySquadInvites(token: string): Promise<SquadInviteResponse[]> {
  return requestJson<SquadInviteResponse[]>('/api/user/me/squad-invites', {
    headers: authHeaders(token),
  })
}

export function getMyWallet(token: string): Promise<WalletBalanceResponse[]> {
  return requestJson<WalletBalanceResponse[]>('/api/user/me/wallet', {
    headers: authHeaders(token),
  })
}

export function getRestrictionMeta(): Promise<RestrictionMetaResponse[]> {
  return requestJson<RestrictionMetaResponse[]>('/api/meta/restrictions')
}

export function getSquadConfig(): Promise<SquadConfigResponse> {
  return requestJson<SquadConfigResponse>('/api/meta/squads/config')
}

export function getSquadMembers(token: string, squadId: string): Promise<SquadMemberResponse[]> {
  return requestJson<SquadMemberResponse[]>(`/api/squads/${squadId}/members`, {
    headers: authHeaders(token),
  })
}
