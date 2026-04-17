import { authHeaders, request } from './http'

export interface LootboxDefinitionResponse {
  id: string
  assetDefinitionId: string
  assetKey: string
  assetDisplayName: string
  assetDescription: string | null
  isPublic: boolean
  isActive: boolean
  metadata: unknown
  createdAt: string
  updatedAt: string
}

export interface LootboxDropResponse {
  id: string
  rewardAssetDefinitionId: string
  rewardAssetKey: string
  rewardAssetDisplayName: string
  rewardOwnershipModel: string
  amount: number | null
  durationSeconds: number | null
  weight: number
  totalWeight: number
  titleI18n: unknown
  isActive: boolean
  sortOrder: number
  createdAt: string
  updatedAt: string
}

export interface LootboxDetailResponse {
  definition: LootboxDefinitionResponse
  drops: LootboxDropResponse[]
}

export interface LootboxRewardResponse {
  assetKey: string
  displayName: string
  title: string
  ownershipModel: string
  amount: number | null
  durationSeconds: number | null
  expiresAt: string | null
}

export interface LootboxOpenHistoryResponse {
  id: number
  userId: string
  lootboxAssetKey: string
  reward: LootboxRewardResponse
  actorKind: string
  actorUserId: string | null
  actorServiceName: string | null
  feedLength: number
  winnerIndex: number
  openedAt: string
}

export interface OwnedLootboxResponse {
  lootboxId: string
  assetKey: string
  displayName: string
  amount: number
  isOpenable: boolean
}

export interface CreateLootboxDefinitionInput {
  asset_key: string
  is_active?: boolean | null
  metadata?: unknown
}

export interface UpdateLootboxDefinitionInput {
  is_active?: boolean | null
  metadata?: unknown
}

export interface CreateLootboxDropInput {
  reward_asset_key: string
  amount?: number | null
  duration_seconds?: number | null
  weight: number
  title_i18n?: unknown
  is_active?: boolean | null
  sort_order?: number | null
}

export interface UpdateLootboxDropInput {
  amount?: number | null
  duration_seconds?: number | null
  weight?: number | null
  title_i18n?: unknown
  is_active?: boolean | null
  sort_order?: number | null
}

export interface OkResponse {
  ok: boolean
}

export function listAdminLootboxes(token: string): Promise<LootboxDefinitionResponse[]> {
  return request<LootboxDefinitionResponse[]>('/api/admin/lootboxes', {
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function listPublicLootboxes(): Promise<LootboxDefinitionResponse[]> {
  return request<LootboxDefinitionResponse[]>('/api/lootboxes', {
    headers: {
      'Content-Type': 'application/json',
    },
  })
}

export function getPublicLootbox(lootboxId: string): Promise<LootboxDetailResponse> {
  return request<LootboxDetailResponse>(`/api/lootboxes/${encodeURIComponent(lootboxId)}`, {
    headers: {
      'Content-Type': 'application/json',
    },
  })
}

export function createAdminLootbox(
  token: string,
  body: CreateLootboxDefinitionInput,
): Promise<LootboxDetailResponse> {
  return request<LootboxDetailResponse>('/api/admin/lootboxes', {
    method: 'POST',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
    body: JSON.stringify(body),
  })
}

export function getAdminLootbox(token: string, lootboxId: string): Promise<LootboxDetailResponse> {
  return request<LootboxDetailResponse>(`/api/admin/lootboxes/${encodeURIComponent(lootboxId)}`, {
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function patchAdminLootbox(
  token: string,
  lootboxId: string,
  body: UpdateLootboxDefinitionInput,
): Promise<LootboxDetailResponse> {
  return request<LootboxDetailResponse>(`/api/admin/lootboxes/${encodeURIComponent(lootboxId)}`, {
    method: 'PATCH',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
    body: JSON.stringify(body),
  })
}

export function createAdminLootboxDrop(
  token: string,
  lootboxId: string,
  body: CreateLootboxDropInput,
): Promise<LootboxDetailResponse> {
  return request<LootboxDetailResponse>(`/api/admin/lootboxes/${encodeURIComponent(lootboxId)}/drops`, {
    method: 'POST',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
    body: JSON.stringify(body),
  })
}

export function patchAdminLootboxDrop(
  token: string,
  lootboxId: string,
  dropId: string,
  body: UpdateLootboxDropInput,
): Promise<LootboxDetailResponse> {
  return request<LootboxDetailResponse>(
    `/api/admin/lootboxes/${encodeURIComponent(lootboxId)}/drops/${encodeURIComponent(dropId)}`,
    {
      method: 'PATCH',
      headers: authHeaders(token, {
        'Content-Type': 'application/json',
      }),
      body: JSON.stringify(body),
    },
  )
}

export function deleteAdminLootboxDrop(
  token: string,
  lootboxId: string,
  dropId: string,
): Promise<OkResponse> {
  return request<OkResponse>(
    `/api/admin/lootboxes/${encodeURIComponent(lootboxId)}/drops/${encodeURIComponent(dropId)}`,
    {
      method: 'DELETE',
      headers: authHeaders(token, {
        'Content-Type': 'application/json',
      }),
    },
  )
}

export function getAdminUserLootboxOpenHistory(
  token: string,
  userId: string,
): Promise<LootboxOpenHistoryResponse[]> {
  return request<LootboxOpenHistoryResponse[]>(
    `/api/admin/users/${encodeURIComponent(userId)}/lootboxes/open-history`,
    {
      headers: authHeaders(token, {
        'Content-Type': 'application/json',
      }),
    },
  )
}

export function getMyLootboxOpenHistory(token: string): Promise<LootboxOpenHistoryResponse[]> {
  return request<LootboxOpenHistoryResponse[]>('/api/user/me/lootboxes/open-history', {
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function getMyLootboxes(token: string): Promise<OwnedLootboxResponse[]> {
  return request<OwnedLootboxResponse[]>('/api/user/me/lootboxes', {
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}
