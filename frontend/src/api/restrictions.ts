import { authHeaders, request } from './http'

export interface UserRestrictionResponse {
  key: string
  createdAt: string
  reason: string | null
}

export function getMyRestrictions(token: string): Promise<UserRestrictionResponse[]> {
  return request<UserRestrictionResponse[]>('/api/user/me/restrictions', {
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}
