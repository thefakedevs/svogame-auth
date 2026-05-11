import { authHeaders, request, requestBlob } from './http'

export interface LittlemiceCheckListItemResponse {
  id: string
  playerUuid: string
  serviceSystemName: string
  status: string
  failureReason: string | null
  requestedAt: string
  receivedAt: string | null
  completedAt: string | null
  expiresAt: string
}

export interface LittlemiceCheckListResponse {
  items: LittlemiceCheckListItemResponse[]
  total: number
  page: number
  perPage: number
  totalPages: number
}

export interface LittlemiceCheckDetailResponse extends LittlemiceCheckListItemResponse {
  serviceTokenId: string
  screenshotSizeBytes: number | null
  screenshot2SizeBytes: number | null
  logSizeBytes: number | null
  clientInfoText: string | null
  clientInfoSizeBytes: number | null
  screenshotUrl: string | null
  screenshot2Url: string | null
  logUrl: string | null
}

export function listAdminUserLittlemiceChecks(
  token: string,
  userId: string,
  query?: {
    page?: number
    perPage?: number
  },
): Promise<LittlemiceCheckListResponse> {
  const params = new URLSearchParams()
  if (query?.page) params.set('page', String(query.page))
  if (query?.perPage) params.set('perPage', String(query.perPage))

  const suffix = params.toString()
  return request<LittlemiceCheckListResponse>(
    `/api/admin/users/${encodeURIComponent(userId)}/littlemice-checks${suffix ? `?${suffix}` : ''}`,
    {
      headers: authHeaders(token, {
        'Content-Type': 'application/json',
      }),
    },
  )
}

export function getAdminLittlemiceCheck(
  token: string,
  checkId: string,
): Promise<LittlemiceCheckDetailResponse> {
  return request<LittlemiceCheckDetailResponse>(
    `/api/admin/littlemice/checks/${encodeURIComponent(checkId)}`,
    {
      headers: authHeaders(token, {
        'Content-Type': 'application/json',
      }),
    },
  )
}

export function getAdminLittlemiceTextContent(token: string, url: string): Promise<string> {
  return request<string>(url, {
    parseAs: 'text',
    headers: authHeaders(token),
  })
}

export function getAdminLittlemiceBinaryContent(token: string, url: string): Promise<Blob> {
  return requestBlob(url, {
    headers: authHeaders(token),
  })
}
