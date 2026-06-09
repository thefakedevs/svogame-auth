import { authHeaders, request } from './http'

export type ReferralSource = 'link' | 'manual'

export interface ReferralRewardResponse {
  assetKey: string
  assetDisplayName: string | null
  assetKind: string
  ownershipModel: string
  isCurrency: boolean
  amount: number | null
  durationSeconds: number | null
  metadata: unknown
}

export interface ReferralPreviewResponse {
  code: string
  title: string
  contentCreatorUserId: string | null
  rewards: ReferralRewardResponse[]
}

export interface ReferralStatsBucket {
  date: string
  registrations: number
}

export interface ReferralStatsResponse {
  total: number
  buckets: ReferralStatsBucket[]
}

export function getReferralPreview(code: string): Promise<ReferralPreviewResponse> {
  return request<ReferralPreviewResponse>(`/api/referrals/${encodeURIComponent(code)}`, {
    headers: {
      'Content-Type': 'application/json',
    },
  })
}

export function getMyReferralStats(
  token: string,
  query: {
    from: string
    to: string
  },
): Promise<ReferralStatsResponse> {
  const params = new URLSearchParams()
  params.set('from', query.from)
  params.set('to', query.to)

  return request<ReferralStatsResponse>(`/api/referrals/me/stats?${params.toString()}`, {
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}
