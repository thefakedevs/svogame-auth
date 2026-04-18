import { authHeaders, request, requestBlob } from './http'

export interface ShopProductLocaleResponse {
  locale: string
  name: string
  description?: string | null
}

export interface ShopProductResponse {
  id: string
  key: string
  assetDefinitionId: string
  assetKey: string
  assetDisplayName: string
  ownershipModel: string
  priceRub: number
  stackableAmount?: number | null
  durationSeconds?: number | null
  maxPerPurchase?: number | null
  maxOwnedAmount?: number | null
  localizedName: string
  localizedDescription?: string | null
  locales: ShopProductLocaleResponse[]
  isActive: boolean
  isPublic: boolean
  isAvailableNow: boolean
  sortOrder: number
  startsAt?: string | null
  endsAt?: string | null
  metadata: unknown
  createdAt: string
  updatedAt: string
}

export interface CreateShopOrderInput {
  product_key: string
  quantity?: number | null
  locale?: string | null
}

export interface ShopPaymentAttemptResponse {
  id: string
  provider: string
  providerPaymentId: string
  checkoutToken?: string | null
  checkoutUrl?: string | null
  status: string
  createdAt: string
  updatedAt: string
}

export interface ShopReceiptResponse {
  id: string
  provider: string
  status: string
  displayStatus: string
  receiptUuid?: string | null
  printUrl?: string | null
  jsonUrl?: string | null
  failureProblem?: string | null
  attemptCount: number
  nextAttemptAt?: string | null
  deadlineAt?: string | null
  completedAt?: string | null
  createdAt: string
  updatedAt: string
}

export interface ShopOrderResponse {
  id: string
  userId: string
  productId: string
  productKey: string
  productName: string
  productDescription?: string | null
  productLocale?: string | null
  assetDefinitionId: string
  assetKey: string
  ownershipModel: string
  quantity: number
  unitPriceRub: number
  totalPriceRub: number
  grantedAmount?: number | null
  grantedDurationSeconds?: number | null
  maxOwnedAmount?: number | null
  paymentProvider: string
  status: string
  failureProblem?: string | null
  paymentExpiresAt?: string | null
  paidAt?: string | null
  fulfilledAt?: string | null
  payment?: ShopPaymentAttemptResponse | null
  receipt?: ShopReceiptResponse | null
  metadata: unknown
  createdAt: string
  updatedAt: string
}

export function listPublicShopProducts(locale?: string | null): Promise<ShopProductResponse[]> {
  const params = new URLSearchParams()
  if (locale) params.set('locale', locale)

  const suffix = params.toString()
  return request<ShopProductResponse[]>(`/api/shop/products${suffix ? `?${suffix}` : ''}`, {
    headers: {
      'Content-Type': 'application/json',
    },
  })
}

export function createMyShopOrder(
  token: string,
  body: CreateShopOrderInput,
): Promise<ShopOrderResponse> {
  return request<ShopOrderResponse>('/api/user/me/shop/orders', {
    method: 'POST',
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
    body: JSON.stringify(body),
  })
}

export function listMyShopOrders(token: string): Promise<ShopOrderResponse[]> {
  return request<ShopOrderResponse[]>('/api/user/me/shop/orders', {
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function getMyShopOrder(token: string, orderId: string): Promise<ShopOrderResponse> {
  return request<ShopOrderResponse>(`/api/user/me/shop/orders/${encodeURIComponent(orderId)}`, {
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function getMyShopOrderReceipt(token: string, orderId: string): Promise<ShopReceiptResponse> {
  return request<ShopReceiptResponse>(`/api/user/me/shop/orders/${encodeURIComponent(orderId)}/receipt`, {
    headers: authHeaders(token, {
      'Content-Type': 'application/json',
    }),
  })
}

export function getMyShopOrderReceiptPrintImage(token: string, orderId: string): Promise<Blob> {
  return requestBlob(`/api/user/me/shop/orders/${encodeURIComponent(orderId)}/receipt/print`, {
    headers: authHeaders(token),
  })
}
