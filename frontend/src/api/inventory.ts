import { ApiError, authHeaders, request } from "./http";

export type OwnershipModel = "stackable" | "entitlement" | "expirable";
export type AssetKind =
  | "currency"
  | "subscription"
  | "lootbox"
  | "item"
  | "skin"
  | "cosmetic"
  | "ticket"
  | "token"
  | string;
export type SkinRarity = "common" | "rare" | "legendary";

export interface AssetResponse {
  id: string;
  key: string;
  displayName: string;
  description: string | null;
  imageUrl?: string | null;
  assetKind: AssetKind;
  ownershipModel: OwnershipModel;
  rarity?: SkinRarity | null;
  weaponKey?: string | null;
  isCurrency: boolean;
  isUserPurchasable: boolean;
  isPublic: boolean;
  isActive: boolean;
  modelUrl?: string | null;
  textureUrl?: string | null;
  metadata: unknown;
  createdAt: string;
  updatedAt: string;
}

export interface AssetListResponse {
  items: AssetResponse[];
  total: number;
  page: number;
  perPage: number;
  totalPages: number;
}

export interface StackableResponse {
  assetKey: string;
  assetDefinitionId: string;
  amount: number;
  updatedAt: string;
}

export interface EntitlementResponse {
  assetKey: string;
  assetDefinitionId: string;
  grantedAt: string;
  updatedAt: string;
}

export interface ExpirableResponse {
  assetKey: string;
  assetDefinitionId: string;
  expiresAt: string;
  grantedAt: string;
  updatedAt: string;
  isActive: boolean;
  lastExtendedAt: string | null;
}

export interface InventoryResponse {
  userId: string;
  stackables: StackableResponse[];
  entitlements: EntitlementResponse[];
  expirables: ExpirableResponse[];
}

export interface WalletBalanceResponse {
  userId: string;
  currencyAssetDefinitionId: string;
  currencyKey: string;
  balance: number;
  updatedAt: string;
}

export interface WalletTransactionResponse {
  id: number;
  currencyAssetDefinitionId: string;
  currencyKey: string;
  operationType: string;
  actorKind: string;
  actorUserId: string | null;
  actorServiceName: string | null;
  delta: number;
  balanceAfter: number;
  metadata: unknown;
  reasonCode: string | null;
  reasonText: string | null;
  createdAt: string;
}

export interface InventoryOperationResponse {
  id: number;
  assetDefinitionId: string;
  assetKey: string;
  ownershipModel: string;
  operationType: string;
  actorKind: string;
  actorUserId: string | null;
  actorServiceName: string | null;
  deltaAmount: number | null;
  newAmount: number | null;
  previousExpiresAt: string | null;
  newExpiresAt: string | null;
  metadata: unknown;
  reasonCode: string | null;
  reasonText: string | null;
  createdAt: string;
}

export interface MutationBody {
  amount?: number | null;
  durationSeconds?: number | null;
  expiresAt?: string | null;
  metadata?: unknown;
  reasonCode?: string | null;
  reasonText?: string | null;
}

export interface OkResponse {
  ok: boolean;
}

export interface SelectedGunskinResponse {
  weaponKey: string;
  assetDefinitionId: string;
  assetKey: string;
  displayName: string;
  description?: string | null;
  rarity: SkinRarity;
  selectedAt: string;
  updatedAt: string;
}

export interface GunskinSelectionListItemResponse {
  weaponKey: string;
  selected: SelectedGunskinResponse;
}

export interface GunskinAssetResponse {
  id: string;
  key: string;
  displayName: string;
  description?: string | null;
  weaponKey: string;
  rarity: SkinRarity;
  ownershipModel: string;
}

export interface GunskinCollectionResponse {
  weaponKey: string;
  selected?: SelectedGunskinResponse | null;
  available: GunskinAssetResponse[];
}

export interface SelectGunskinInput {
  assetKey: string;
  metadata?: unknown;
  reasonCode?: string | null;
  reasonText?: string | null;
}

export interface CreateAssetDefinitionInput {
  key: string;
  display_name: string;
  asset_kind: AssetKind;
  ownership_model: OwnershipModel;
  is_currency: boolean;
  is_user_purchasable: boolean;
  is_public: boolean;
  description?: string | null;
  rarity?: SkinRarity | null;
  weaponKey?: string | null;
  metadata?: unknown;
}

export interface UpdateAssetDefinitionInput {
  display_name?: string | null;
  description?: string | null;
  is_active?: boolean | null;
  is_public?: boolean | null;
  is_user_purchasable?: boolean | null;
  rarity?: SkinRarity | null;
  weaponKey?: string | null;
  metadata?: unknown;
}

function appendImageVersion(url: string, version?: string | null) {
  return version ? `${url}?v=${encodeURIComponent(version)}` : url;
}

export function buildPublicAssetImageUrl(assetId: string, version?: string | null) {
  return appendImageVersion(`/api/assets/${encodeURIComponent(assetId)}/image`, version);
}

export function buildAdminAssetImageUrl(assetId: string, version?: string | null) {
  return appendImageVersion(`/api/admin/assets/${encodeURIComponent(assetId)}/image`, version);
}

export async function fetchAdminAssetImageObjectUrl(
  token: string,
  assetId: string,
  version?: string | null,
): Promise<string | null> {
  let response: Response;
  try {
    response = await fetch(buildAdminAssetImageUrl(assetId, version), {
      headers: authHeaders(token),
    });
  } catch (error) {
    throw new ApiError({
      kind: "network",
      message: "Не удалось загрузить изображение ассета.",
      details: error,
    });
  }

  if (response.status === 404) {
    return null;
  }

  if (!response.ok) {
    let details: unknown = null;
    try {
      details = await response.text();
    } catch {
      details = null;
    }

    throw new ApiError({
      kind: "http",
      status: response.status,
      statusText: response.statusText,
      message: response.statusText || "Не удалось загрузить изображение ассета.",
      details,
    });
  }

  const blob = await response.blob();
  return blob.size > 0 ? URL.createObjectURL(blob) : null;
}

export function listPublicAssets(query?: {
  q?: string;
  assetKind?: string;
  ownershipModel?: string;
  isCurrency?: boolean;
  isUserPurchasable?: boolean;
  page?: number;
  perPage?: number;
}): Promise<AssetListResponse> {
  const params = new URLSearchParams();
  if (query?.q) params.set("q", query.q);
  if (query?.assetKind) params.set("assetKind", query.assetKind);
  if (query?.ownershipModel) params.set("ownershipModel", query.ownershipModel);
  if (query?.isCurrency !== undefined) params.set("isCurrency", String(query.isCurrency));
  if (query?.isUserPurchasable !== undefined)
    params.set("isUserPurchasable", String(query.isUserPurchasable));
  if (query?.page) params.set("page", String(query.page));
  if (query?.perPage) params.set("perPage", String(query.perPage));

  const suffix = params.toString();
  return request<AssetListResponse>(`/api/assets${suffix ? `?${suffix}` : ""}`, {
    headers: {
      "Content-Type": "application/json",
    },
  });
}

export async function listAllPublicAssets(perPage = 100): Promise<AssetResponse[]> {
  const items: AssetResponse[] = [];
  let page = 1;

  while (true) {
    const response = await listPublicAssets({ page, perPage });
    items.push(...response.items);

    if (items.length >= response.total || response.items.length === 0) {
      return items;
    }

    page += 1;
  }
}

export function getMyInventory(token: string): Promise<InventoryResponse> {
  return request<InventoryResponse>("/api/user/me/inventory", {
    headers: authHeaders(token, {
      "Content-Type": "application/json",
    }),
  });
}

export function listMyEntitlements(token: string): Promise<EntitlementResponse[]> {
  return request<EntitlementResponse[]>("/api/user/me/inventory/entitlements", {
    headers: authHeaders(token, {
      "Content-Type": "application/json",
    }),
  });
}

export function getMyWallet(token: string): Promise<WalletBalanceResponse[]> {
  return request<WalletBalanceResponse[]>("/api/user/me/wallet", {
    headers: authHeaders(token, {
      "Content-Type": "application/json",
    }),
  });
}

export function getMyDefaultWalletBalance(token: string): Promise<WalletBalanceResponse> {
  return request<WalletBalanceResponse>("/api/user/me/wallet/default", {
    headers: authHeaders(token, {
      "Content-Type": "application/json",
    }),
  });
}

export function getMyWalletTransactions(
  token: string,
  currencyKey: string,
): Promise<WalletTransactionResponse[]> {
  return request<WalletTransactionResponse[]>(
    `/api/user/me/wallet/${encodeURIComponent(currencyKey)}/transactions`,
    {
      headers: authHeaders(token, {
        "Content-Type": "application/json",
      }),
    },
  );
}

export async function getMyDefaultWalletTransactions(
  token: string,
): Promise<WalletTransactionResponse[]> {
  const balance = await getMyDefaultWalletBalance(token);
  return getMyWalletTransactions(token, balance.currencyKey);
}

export function listMyGunskinSelections(
  token: string,
): Promise<GunskinSelectionListItemResponse[]> {
  return request<GunskinSelectionListItemResponse[]>("/api/user/me/gunskins/selections", {
    headers: authHeaders(token, {
      "Content-Type": "application/json",
    }),
  });
}

export function getMyGunskinCollection(
  token: string,
  weaponKey: string,
): Promise<GunskinCollectionResponse> {
  return request<GunskinCollectionResponse>(
    `/api/user/me/gunskins/${encodeURIComponent(weaponKey)}`,
    {
      headers: authHeaders(token, {
        "Content-Type": "application/json",
      }),
    },
  );
}

export function selectMyGunskin(
  token: string,
  weaponKey: string,
  body: SelectGunskinInput,
): Promise<SelectedGunskinResponse> {
  return request<SelectedGunskinResponse>(
    `/api/user/me/gunskins/${encodeURIComponent(weaponKey)}/selected`,
    {
      method: "PUT",
      headers: authHeaders(token, {
        "Content-Type": "application/json",
      }),
      body: JSON.stringify(body),
    },
  );
}

export function resetMyGunskin(token: string, weaponKey: string): Promise<OkResponse> {
  return request<OkResponse>(`/api/user/me/gunskins/${encodeURIComponent(weaponKey)}/selected`, {
    method: "DELETE",
    headers: authHeaders(token, {
      "Content-Type": "application/json",
    }),
  });
}

export function listAdminAssets(
  token: string,
  query?: {
    q?: string;
    assetKind?: string;
    ownershipModel?: string;
    isCurrency?: boolean;
    isPublic?: boolean;
    isUserPurchasable?: boolean;
    isActive?: boolean;
    page?: number;
    perPage?: number;
  },
): Promise<AssetListResponse> {
  const params = new URLSearchParams();
  if (query?.q) params.set("q", query.q);
  if (query?.assetKind) params.set("assetKind", query.assetKind);
  if (query?.ownershipModel) params.set("ownershipModel", query.ownershipModel);
  if (query?.isCurrency !== undefined) params.set("isCurrency", String(query.isCurrency));
  if (query?.isPublic !== undefined) params.set("isPublic", String(query.isPublic));
  if (query?.isUserPurchasable !== undefined)
    params.set("isUserPurchasable", String(query.isUserPurchasable));
  if (query?.isActive !== undefined) params.set("isActive", String(query.isActive));
  if (query?.page) params.set("page", String(query.page));
  if (query?.perPage) params.set("perPage", String(query.perPage));

  const suffix = params.toString();
  return request<AssetListResponse>(`/api/admin/assets${suffix ? `?${suffix}` : ""}`, {
    headers: authHeaders(token, {
      "Content-Type": "application/json",
    }),
  });
}

export async function listAllAdminAssets(token: string, perPage = 100): Promise<AssetResponse[]> {
  const items: AssetResponse[] = [];
  let page = 1;

  while (true) {
    const response = await listAdminAssets(token, { page, perPage });
    items.push(...response.items);

    if (items.length >= response.total || response.items.length === 0) {
      return items;
    }

    page += 1;
  }
}

export function createAdminAsset(
  token: string,
  body: CreateAssetDefinitionInput,
): Promise<AssetResponse> {
  return request<AssetResponse>("/api/admin/assets", {
    method: "POST",
    headers: authHeaders(token, {
      "Content-Type": "application/json",
    }),
    body: JSON.stringify(body),
  });
}

export function getAdminAsset(token: string, assetId: string): Promise<AssetResponse> {
  return request<AssetResponse>(`/api/admin/assets/${encodeURIComponent(assetId)}`, {
    headers: authHeaders(token, {
      "Content-Type": "application/json",
    }),
  });
}

export function patchAdminAsset(
  token: string,
  assetId: string,
  body: UpdateAssetDefinitionInput,
): Promise<AssetResponse> {
  return request<AssetResponse>(`/api/admin/assets/${assetId}`, {
    method: "PATCH",
    headers: authHeaders(token, {
      "Content-Type": "application/json",
    }),
    body: JSON.stringify(body),
  });
}

export function uploadAdminAssetImage(
  token: string,
  assetId: string,
  file: File,
): Promise<AssetResponse> {
  const formData = new FormData();
  formData.append("image", file);

  return request<AssetResponse>(buildAdminAssetImageUrl(assetId), {
    method: "POST",
    headers: authHeaders(token),
    body: formData,
  });
}

export function deleteAdminAssetImage(token: string, assetId: string): Promise<AssetResponse> {
  return request<AssetResponse>(buildAdminAssetImageUrl(assetId), {
    method: "DELETE",
    headers: authHeaders(token),
  });
}

export function getAdminUserInventory(token: string, userId: string): Promise<InventoryResponse> {
  return request<InventoryResponse>(`/api/admin/users/${userId}/inventory`, {
    headers: authHeaders(token, {
      "Content-Type": "application/json",
    }),
  });
}

export function getAdminUserInventoryHistory(
  token: string,
  userId: string,
): Promise<InventoryOperationResponse[]> {
  return request<InventoryOperationResponse[]>(`/api/admin/users/${userId}/inventory/history`, {
    headers: authHeaders(token, {
      "Content-Type": "application/json",
    }),
  });
}

export function grantAdminEntitlement(
  token: string,
  userId: string,
  assetKey: string,
  body: MutationBody,
): Promise<EntitlementResponse> {
  return request<EntitlementResponse>(
    `/api/admin/users/${userId}/inventory/entitlements/${encodeURIComponent(assetKey)}/grant`,
    {
      method: "POST",
      headers: authHeaders(token, {
        "Content-Type": "application/json",
      }),
      body: JSON.stringify(body),
    },
  );
}

export function revokeAdminEntitlement(
  token: string,
  userId: string,
  assetKey: string,
  body: MutationBody,
): Promise<OkResponse> {
  return request<OkResponse>(
    `/api/admin/users/${userId}/inventory/entitlements/${encodeURIComponent(assetKey)}/revoke`,
    {
      method: "POST",
      headers: authHeaders(token, {
        "Content-Type": "application/json",
      }),
      body: JSON.stringify(body),
    },
  );
}

export function addAdminStackable(
  token: string,
  userId: string,
  assetKey: string,
  body: MutationBody,
): Promise<StackableResponse> {
  return request<StackableResponse>(
    `/api/admin/users/${userId}/inventory/stackables/${encodeURIComponent(assetKey)}/add`,
    {
      method: "POST",
      headers: authHeaders(token, {
        "Content-Type": "application/json",
      }),
      body: JSON.stringify(body),
    },
  );
}

export function removeAdminStackable(
  token: string,
  userId: string,
  assetKey: string,
  body: MutationBody,
): Promise<StackableResponse> {
  return request<StackableResponse>(
    `/api/admin/users/${userId}/inventory/stackables/${encodeURIComponent(assetKey)}/remove`,
    {
      method: "POST",
      headers: authHeaders(token, {
        "Content-Type": "application/json",
      }),
      body: JSON.stringify(body),
    },
  );
}

export function setAdminStackable(
  token: string,
  userId: string,
  assetKey: string,
  body: MutationBody,
): Promise<StackableResponse> {
  return request<StackableResponse>(
    `/api/admin/users/${userId}/inventory/stackables/${encodeURIComponent(assetKey)}`,
    {
      method: "PUT",
      headers: authHeaders(token, {
        "Content-Type": "application/json",
      }),
      body: JSON.stringify(body),
    },
  );
}

export function prolongAdminExpirable(
  token: string,
  userId: string,
  assetKey: string,
  body: MutationBody,
): Promise<ExpirableResponse> {
  return request<ExpirableResponse>(
    `/api/admin/users/${userId}/inventory/expirables/${encodeURIComponent(assetKey)}/prolong`,
    {
      method: "POST",
      headers: authHeaders(token, {
        "Content-Type": "application/json",
      }),
      body: JSON.stringify(body),
    },
  );
}

export function setAdminExpirableExpiration(
  token: string,
  userId: string,
  assetKey: string,
  body: MutationBody,
): Promise<ExpirableResponse> {
  return request<ExpirableResponse>(
    `/api/admin/users/${userId}/inventory/expirables/${encodeURIComponent(assetKey)}/expiration`,
    {
      method: "PUT",
      headers: authHeaders(token, {
        "Content-Type": "application/json",
      }),
      body: JSON.stringify(body),
    },
  );
}

export function revokeAdminExpirable(
  token: string,
  userId: string,
  assetKey: string,
  body: MutationBody,
): Promise<OkResponse> {
  return request<OkResponse>(
    `/api/admin/users/${userId}/inventory/expirables/${encodeURIComponent(assetKey)}`,
    {
      method: "DELETE",
      headers: authHeaders(token, {
        "Content-Type": "application/json",
      }),
      body: JSON.stringify(body),
    },
  );
}

export function getAdminUserDefaultWalletBalance(
  token: string,
  userId: string,
): Promise<WalletBalanceResponse> {
  return request<WalletBalanceResponse>(`/api/admin/users/${userId}/wallet/default`, {
    headers: authHeaders(token, {
      "Content-Type": "application/json",
    }),
  });
}

export function getAdminUserWalletTransactions(
  token: string,
  userId: string,
  currencyKey: string,
): Promise<WalletTransactionResponse[]> {
  return request<WalletTransactionResponse[]>(
    `/api/admin/users/${userId}/wallet/${encodeURIComponent(currencyKey)}/transactions`,
    {
      headers: authHeaders(token, {
        "Content-Type": "application/json",
      }),
    },
  );
}

export function creditAdminDefaultWallet(
  token: string,
  userId: string,
  body: MutationBody,
): Promise<WalletBalanceResponse> {
  return request<WalletBalanceResponse>(`/api/admin/users/${userId}/wallet/default/credit`, {
    method: "POST",
    headers: authHeaders(token, {
      "Content-Type": "application/json",
    }),
    body: JSON.stringify(body),
  });
}

export function debitAdminDefaultWallet(
  token: string,
  userId: string,
  body: MutationBody,
): Promise<WalletBalanceResponse> {
  return request<WalletBalanceResponse>(`/api/admin/users/${userId}/wallet/default/debit`, {
    method: "POST",
    headers: authHeaders(token, {
      "Content-Type": "application/json",
    }),
    body: JSON.stringify(body),
  });
}

export function adjustAdminDefaultWallet(
  token: string,
  userId: string,
  body: MutationBody,
): Promise<WalletBalanceResponse> {
  return request<WalletBalanceResponse>(`/api/admin/users/${userId}/wallet/default`, {
    method: "PUT",
    headers: authHeaders(token, {
      "Content-Type": "application/json",
    }),
    body: JSON.stringify(body),
  });
}
