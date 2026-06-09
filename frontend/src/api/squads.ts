import { ApiError, authHeaders, request, requestNullable } from './http'
import type { SquadConfigResponse } from './meta'

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
  inviteId: string | null
  isPendingInvite: boolean
}

export interface SquadInviteResponse {
  id: string
  squadId: string
  squadName: string
  inviterUserId: string
  invitedUserId: string
  expiresAt: string
  createdAt: string
  invitedUsername: string
  inviterAvatarUrl: string | null
  inviterUsername: string
}

export interface SquadActionResponse {
  status: string
}

export interface UserSearchItemResponse {
  id: string
  username: string
  avatarUrl: string | null
}

export type { SquadConfigResponse }

export function getMySquad(token: string): Promise<SquadResponse | null> {
  return requestNullable<SquadResponse>('/api/user/me/squad', {
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  }).catch((error) => {
    if (error instanceof ApiError && error.status === 400 && error.message.includes('Invalid squad ID')) {
      return null
    }
    throw error
  })
}

export function getMySquadInvites(token: string): Promise<SquadInviteResponse[]> {
  return request<SquadInviteResponse[]>('/api/user/me/squad-invites', {
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function getSquadMembers(token: string, squadId: string): Promise<SquadMemberResponse[]> {
  return request<SquadMemberResponse[]>(`/api/squads/${squadId}/members`, {
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function createSquad(token: string, name: string): Promise<SquadResponse> {
  return request<SquadResponse>('/api/squads', {
    method: 'POST',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
    body: JSON.stringify({ name }),
  })
}

export function patchSquad(token: string, squadId: string, name: string): Promise<SquadResponse> {
  return request<SquadResponse>(`/api/squads/${squadId}`, {
    method: 'PATCH',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
    body: JSON.stringify({ name }),
  })
}

export function uploadSquadImage(token: string, squadId: string, file: File): Promise<SquadResponse> {
  const formData = new FormData()
  formData.append('file', file)

  return request<SquadResponse>(`/api/squads/${squadId}/image`, {
    method: 'POST',
    headers: authHeaders(token),
    body: formData,
  })
}

export function deleteSquad(token: string, squadId: string): Promise<SquadActionResponse> {
  return request<SquadActionResponse>(`/api/squads/${squadId}`, {
    method: 'DELETE',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function leaveSquad(token: string, squadId: string): Promise<SquadActionResponse> {
  return request<SquadActionResponse>(`/api/squads/${squadId}/leave`, {
    method: 'POST',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function kickSquadMember(token: string, squadId: string, userId: string): Promise<SquadActionResponse> {
  return request<SquadActionResponse>(`/api/squads/${squadId}/members/${userId}/kick`, {
    method: 'POST',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function createSquadInvite(token: string, squadId: string, userId: string): Promise<SquadInviteResponse> {
  return request<SquadInviteResponse>(`/api/squads/${squadId}/invites`, {
    method: 'POST',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
    body: JSON.stringify({ userId }),
  })
}

export function revokeSquadInvite(token: string, inviteId: string): Promise<SquadActionResponse> {
  return request<SquadActionResponse>(`/api/squad-invites/${inviteId}`, {
    method: 'DELETE',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function acceptSquadInvite(token: string, inviteId: string): Promise<SquadActionResponse> {
  return request<SquadActionResponse>(`/api/squad-invites/${inviteId}/accept`, {
    method: 'POST',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function declineSquadInvite(token: string, inviteId: string): Promise<SquadActionResponse> {
  return request<SquadActionResponse>(`/api/squad-invites/${inviteId}/decline`, {
    method: 'POST',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function searchUsers(token: string, q: string, limit = 10): Promise<UserSearchItemResponse[]> {
  const params = new URLSearchParams({
    q,
    limit: String(limit),
  })

  return request<UserSearchItemResponse[]>(`/api/users/search?${params.toString()}`, {
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}
