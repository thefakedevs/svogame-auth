import { authHeaders, request } from './http'
import type { SquadMemberResponse } from './squads'

export interface AdminMeResponse {
  id: string
  username: string
  isSuperuser: boolean
}

export interface AdminUserResponse {
  id: string
  discordId: string
  username: string
  avatarUrl: string | null
  email: string | null
  authEpoch: number
  isActive: boolean
  isSuperuser: boolean
  lastLoginAt: string
  createdAt: string
  deactivationReason: string | null
}

export interface AdminUserRestrictionResponse {
  key: string
  reason: string | null
  createdAt: string
}

export interface AdminUsersListResponse {
  items: AdminUserResponse[]
  total: number
  page: number
  perPage: number
  totalPages: number
}

export interface AdminSquadResponse {
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

export interface AdminSquadListResponse {
  items: AdminSquadResponse[]
  total: number
  page: number
  perPage: number
}

export interface ServiceTokenResponse {
  id: string
  systemName: string
  description: string | null
  isActive: boolean
  createdByUserId: string
  createdAt: string
  updatedAt: string
  expiresAt: string | null
  lastUsedAt: string | null
  rotatedFromId: string | null
  plaintextToken: string | null
}

export interface ServiceTokenAuditResponse {
  id: number
  serviceTokenId: string
  action: string
  actorUserId: string
  metadata: unknown
  reason: string | null
  createdAt: string
}

export interface CreateServiceTokenRequest {
  systemName: string
  description?: string | null
  expiresAt?: string | null
}

export interface RotateServiceTokenRequest {
  reason?: string | null
  expiresAt?: string | null
}

export interface RevokeServiceTokenRequest {
  reason?: string | null
}

export interface RestrictionReasonRequest {
  reason?: string | null
}

export interface PatchAdminSquadRequest {
  name?: string | null
  isRestricted?: boolean | null
  restrictionReason?: string | null
}

export interface DeactivateAdminUserRequest {
  reason: string
}

export interface ActionResponse {
  status: string
}

export function getAdminMe(token: string): Promise<AdminMeResponse> {
  return request<AdminMeResponse>('/api/admin/me', {
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function listAdminUsers(
  token: string,
  query?: {
    q?: string
    page?: number
    perPage?: number
  },
): Promise<AdminUsersListResponse> {
  const params = new URLSearchParams()
  if (query?.q) params.set('q', query.q)
  if (query?.page) params.set('page', String(query.page))
  if (query?.perPage) params.set('perPage', String(query.perPage))

  const suffix = params.toString()
  return request<AdminUsersListResponse>(`/api/admin/users${suffix ? `?${suffix}` : ''}`, {
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function getAdminUser(token: string, userId: string): Promise<AdminUserResponse> {
  return request<AdminUserResponse>(`/api/admin/users/${userId}`, {
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function grantAdminUserSuperuser(token: string, userId: string): Promise<AdminUserResponse> {
  return request<AdminUserResponse>(`/api/admin/users/${userId}/grant-superuser`, {
    method: 'POST',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function revokeAdminUserSuperuser(token: string, userId: string): Promise<AdminUserResponse> {
  return request<AdminUserResponse>(`/api/admin/users/${userId}/revoke-superuser`, {
    method: 'POST',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function activateAdminUser(token: string, userId: string): Promise<AdminUserResponse> {
  return request<AdminUserResponse>(`/api/admin/users/${userId}/activate`, {
    method: 'POST',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function deactivateAdminUser(
  token: string,
  userId: string,
  payload: DeactivateAdminUserRequest,
): Promise<AdminUserResponse> {
  return request<AdminUserResponse>(`/api/admin/users/${userId}/deactivate`, {
    method: 'POST',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
    body: JSON.stringify(payload),
  })
}

export function getAdminUserRestrictions(
  token: string,
  userId: string,
): Promise<AdminUserRestrictionResponse[]> {
  return request<AdminUserRestrictionResponse[]>(`/api/admin/users/${userId}/restrictions`, {
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function grantAdminUserRestriction(
  token: string,
  userId: string,
  restrictionKey: string,
  payload: RestrictionReasonRequest,
): Promise<AdminUserRestrictionResponse[]> {
  return request<AdminUserRestrictionResponse[]>(`/api/admin/users/${userId}/restrictions/${encodeURIComponent(restrictionKey)}`, {
    method: 'POST',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
    body: JSON.stringify(payload),
  })
}

export function revokeAdminUserRestriction(
  token: string,
  userId: string,
  restrictionKey: string,
  payload: RestrictionReasonRequest,
): Promise<AdminUserRestrictionResponse[]> {
  return request<AdminUserRestrictionResponse[]>(`/api/admin/users/${userId}/restrictions/${encodeURIComponent(restrictionKey)}`, {
    method: 'DELETE',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
    body: JSON.stringify(payload),
  })
}

export function getAdminUserSquad(token: string, userId: string): Promise<AdminSquadResponse | null> {
  return request<AdminSquadResponse | null>(`/api/admin/user/${userId}/squad`, {
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function deleteAdminUserSkin(token: string, userId: string): Promise<ActionResponse> {
  return request<ActionResponse>(`/api/admin/user/${userId}/skin`, {
    method: 'DELETE',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function listAdminSquads(
  token: string,
  query?: {
    q?: string
    page?: number
    perPage?: number
  },
): Promise<AdminSquadListResponse> {
  const params = new URLSearchParams()
  if (query?.q) params.set('q', query.q)
  if (query?.page) params.set('page', String(query.page))
  if (query?.perPage) params.set('perPage', String(query.perPage))

  const suffix = params.toString()
  return request<AdminSquadListResponse>(`/api/admin/squads${suffix ? `?${suffix}` : ''}`, {
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export async function listAllAdminSquads(token: string, perPage = 100): Promise<AdminSquadResponse[]> {
  const items: AdminSquadResponse[] = []
  let page = 1

  while (true) {
    const response = await listAdminSquads(token, { page, perPage })
    items.push(...response.items)

    if (items.length >= response.total || response.items.length === 0) {
      return items
    }

    page += 1
  }
}

export function getAdminSquad(token: string, squadId: string): Promise<AdminSquadResponse> {
  return request<AdminSquadResponse>(`/api/admin/squads/${squadId}`, {
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function patchAdminSquad(
  token: string,
  squadId: string,
  payload: PatchAdminSquadRequest,
): Promise<AdminSquadResponse> {
  return request<AdminSquadResponse>(`/api/admin/squads/${squadId}`, {
    method: 'PATCH',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
    body: JSON.stringify(payload),
  })
}

export function deleteAdminSquad(token: string, squadId: string): Promise<ActionResponse> {
  return request<ActionResponse>(`/api/admin/squads/${squadId}`, {
    method: 'DELETE',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function restrictAdminSquad(
  token: string,
  squadId: string,
  payload: RestrictionReasonRequest,
): Promise<AdminSquadResponse> {
  return request<AdminSquadResponse>(`/api/admin/squads/${squadId}/restrict`, {
    method: 'POST',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
    body: JSON.stringify(payload),
  })
}

export function unrestrictAdminSquad(
  token: string,
  squadId: string,
  payload: RestrictionReasonRequest,
): Promise<AdminSquadResponse> {
  return request<AdminSquadResponse>(`/api/admin/squads/${squadId}/unrestrict`, {
    method: 'POST',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
    body: JSON.stringify(payload),
  })
}

export function deleteAdminSquadImage(token: string, squadId: string): Promise<AdminSquadResponse> {
  return request<AdminSquadResponse>(`/api/admin/squads/${squadId}/image`, {
    method: 'DELETE',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function listAdminServiceTokens(token: string): Promise<ServiceTokenResponse[]> {
  return request<ServiceTokenResponse[]>('/api/admin/service-tokens', {
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function createAdminServiceToken(
  token: string,
  payload: CreateServiceTokenRequest,
): Promise<ServiceTokenResponse> {
  return request<ServiceTokenResponse>('/api/admin/service-tokens', {
    method: 'POST',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
    body: JSON.stringify(payload),
  })
}

export function rotateAdminServiceToken(
  token: string,
  tokenId: string,
  payload: RotateServiceTokenRequest,
): Promise<ServiceTokenResponse> {
  return request<ServiceTokenResponse>(`/api/admin/service-tokens/${tokenId}/rotate`, {
    method: 'POST',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
    body: JSON.stringify(payload),
  })
}

export function revokeAdminServiceToken(
  token: string,
  tokenId: string,
  payload: RevokeServiceTokenRequest,
): Promise<ServiceTokenResponse> {
  return request<ServiceTokenResponse>(`/api/admin/service-tokens/${tokenId}/revoke`, {
    method: 'POST',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
    body: JSON.stringify(payload),
  })
}

export function getAdminServiceTokenAudit(
  token: string,
  tokenId: string,
): Promise<ServiceTokenAuditResponse[]> {
  return request<ServiceTokenAuditResponse[]>(`/api/admin/service-tokens/${tokenId}/audit`, {
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function getPublicSquadMembers(squadId: string): Promise<SquadMemberResponse[]> {
  return request<SquadMemberResponse[]>(`/api/squads/${squadId}/members`, {
    headers: {
      'Content-Type': 'application/json',
    },
  })
}
