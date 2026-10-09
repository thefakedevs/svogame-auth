import seed from '../../public/static-preview/data.json'
import { withBasePath } from '../shared/navigation/basePath'

export const STATIC_PREVIEW = true

const previewUserId = '00000000-0000-4000-8000-000000000071'
const walletAsset = seed.assets.find((asset) => asset.isCurrency)!
const stamp = '2026-07-20T12:00:00.000Z'
type PreviewSquad = { id: string; name: string; leaderUserId: string; memberCount: number; maxMembers: number; imageUrl: string | null; isRestricted: boolean; restrictionReason: string | null; createdAt: string; updatedAt: string }
type PreviewShopOrder = Record<string, unknown> & { id: string; payment: Record<string, unknown> & { checkoutUrl: string } }
interface PreviewMetricRow {
  playerId: string; nickname: string; matchesPlayed: number; kills: number; deaths: number; assists: number
  vehicleDestructions: number; timeInGameMs: number; kd: number; kda: number; damageDealt: number
  damagePerMinute: number; winRate: number; headshots: number
}
const previewUser = {
  id: previewUserId,
  discordId: 'example-player',
  username: 'Игрок',
  avatarUrl: null,
  email: "example@example.com",
  isActive: true,
  isSuperuser: false,
  lastLoginAt: stamp,
  createdAt: '2025-01-15T12:00:00.000Z',
}

const weapons = [...new Set(seed.assets.map((asset) => asset.weaponKey).filter((value): value is string => Boolean(value)))]
const skinAssets = seed.assets.filter((asset) => asset.assetKind === 'skin')
const selectedAssets = weapons.map((weaponKey, index) => skinAssets.find((asset) => asset.weaponKey === weaponKey && index % 2 === 0) ?? skinAssets.find((asset) => asset.weaponKey === weaponKey)).filter(Boolean)
const selectedSkinByWeapon = new Map(selectedAssets.filter(Boolean).map((asset) => [asset!.weaponKey!, asset!]))
const shopOrders: Record<string, PreviewShopOrder> = {}
let nextOrderNumber = 1
let nickname = previewUser.username
let previewSquad: PreviewSquad | null = {
  id: 'preview-squad',
  name: 'Север',
  leaderUserId: '00000000-0000-4000-8000-000000000072',
  memberCount: 4,
  maxMembers: 10,
  imageUrl: null,
  isRestricted: false,
  restrictionReason: null,
  createdAt: '2025-03-12T12:00:00.000Z',
  updatedAt: stamp,
}
let previewSquadMembers = [
  { id: '00000000-0000-4000-8000-000000000072', username: 'Varyag', avatarUrl: null, isLeader: true, inviteId: null, isPendingInvite: false },
  { id: previewUserId, username: nickname, avatarUrl: null, isLeader: false, inviteId: null, isPendingInvite: false },
  { id: '00000000-0000-4000-8000-000000000073', username: 'Ворон', avatarUrl: null, isLeader: false, inviteId: null, isPendingInvite: false },
  { id: '00000000-0000-4000-8000-000000000074', username: 'Кедр', avatarUrl: null, isLeader: false, inviteId: null, isPendingInvite: false },
]

const currencies = { userId: previewUserId, currencyAssetDefinitionId: walletAsset.id, currencyKey: walletAsset.key, balance: 1250, updatedAt: stamp }

function profileUser() {
  return { ...previewUser, username: nickname }
}

function assetList(url: URL) {
  const params = url.searchParams
  let items = [...seed.assets]
  const q = params.get('q')?.toLowerCase()
  if (q) items = items.filter((asset) => `${asset.key} ${asset.displayName} ${asset.weaponKey ?? ''}`.toLowerCase().includes(q))
  if (params.has('assetKind')) items = items.filter((asset) => asset.assetKind === params.get('assetKind'))
  if (params.has('ownershipModel')) items = items.filter((asset) => asset.ownershipModel === params.get('ownershipModel'))
  if (params.has('isCurrency')) items = items.filter((asset) => asset.isCurrency === (params.get('isCurrency') === 'true'))
  if (params.has('isUserPurchasable')) items = items.filter((asset) => asset.isUserPurchasable === (params.get('isUserPurchasable') === 'true'))
  const page = Math.max(1, Number(params.get('page') ?? 1))
  const perPage = Math.max(1, Number(params.get('perPage') ?? 100))
  const start = (page - 1) * perPage
  return { items: items.slice(start, start + perPage).map((asset) => ({ ...asset, imageUrl: asset.imageUrl ? withBasePath(asset.imageUrl) : asset.imageUrl })), total: items.length, page, perPage, totalPages: Math.ceil(items.length / perPage) }
}

function metricRows(metric: string) {
  const rows: PreviewMetricRow[] = [...seed.leaderboard]
  const fields: Record<string, keyof PreviewMetricRow> = { matches_played: 'matchesPlayed', kills: 'kills', deaths: 'deaths', assists: 'assists', vehicle_destructions: 'vehicleDestructions', kd: 'kd', kda: 'kda', damage_dealt: 'damageDealt', damage_per_minute: 'damagePerMinute', win_rate: 'winRate', headshots: 'headshots', time_in_game: 'timeInGameMs' }
  const field = fields[metric] ?? 'kills'
  return rows.sort((a, b) => Number(b[field]) - Number(a[field]))
}

function profileStats() {
  return { ...seed.profileStats, nickname }
}

function matches() {
  return seed.profileMatches
}

function shopOrder(body: Record<string, unknown>) {
  const product = seed.shopProducts.find((item) => item.key === body.product_key) ?? seed.shopProducts[0]
  if (!product) return null
  const id = `preview-order-${nextOrderNumber++}`
  const quantity = Math.max(1, Number(body.quantity ?? 1))
  const order: PreviewShopOrder = {
    id, userId: previewUserId, productId: product.id, productKey: product.key,
    productName: product.localizedName, productDescription: product.localizedDescription, productLocale: body.locale ?? 'ru-RU',
    assetDefinitionId: product.assetDefinitionId, assetKey: product.assetKey, ownershipModel: product.ownershipModel,
    quantity, unitPriceRub: product.priceRub, totalPriceRub: product.priceRub * quantity,
    grantedAmount: product.stackableAmount, grantedDurationSeconds: product.durationSeconds, maxOwnedAmount: product.maxOwnedAmount,
    paymentProvider: 'mock', status: 'paid', failureProblem: null, paymentExpiresAt: null, paidAt: stamp, fulfilledAt: stamp,
    payment: { id: `${id}-payment`, provider: 'mock', providerPaymentId: id, checkoutToken: null, checkoutUrl: `${withBasePath('/shop/checkout/return')}?orderId=${id}`, status: 'succeeded', createdAt: stamp, updatedAt: stamp },
    receipt: null, metadata: {}, createdAt: stamp, updatedAt: stamp,
  }
  shopOrders[id] = order
  return order
}

export async function mockStaticApi(input: string, init: RequestInit = {}): Promise<unknown> {
  const url = new URL(input, window.location.origin)
  const path = url.pathname
  const method = (init.method ?? 'GET').toUpperCase()
  if (url.hostname === 'api.mcsrvstat.us') return { online: true, players: { online: 247, max: 500 } }
  if (path === '/api/meta/squads/config') return { maxMembers: 10, inviteTtlHours: 72, nameMinChars: 3, nameMaxChars: 24, nameRegex: '^[\\p{L}0-9 _-]+$', imageMaxBytes: 2_097_152, imageMaxWidth: 512, imageMaxHeight: 512 }
  if (path === '/api/meta/restrictions') return []
  if (path === '/api/discord/events') return [{ id: 'preview-event', name: 'Операция «Север»', description: 'Совместный выход на сервер.', status: 'scheduled', startsAt: '2026-07-21T16:00:00.000Z', endsAt: '2026-07-21T18:00:00.000Z', imageUrl: null, location: 'Сервер SvoCraft', userCount: 24 }]
  if (path === '/api/user/me') return profileUser()
  if (path === '/api/user/me/nickname' && method === 'POST') { nickname = JSON.parse(String(init.body ?? '{}')).nickname ?? nickname; return profileUser() }
  if (path === '/api/user/me/squad') return previewSquad
  if (path === '/api/user/me/squad-invites') return []
  if (path === '/api/user/me/restrictions') return []
  if (path === '/api/squads' && method === 'POST') {
    const body = JSON.parse(String(init.body ?? '{}'))
    previewSquad = { id: 'preview-squad', name: typeof body.name === 'string' ? body.name : 'Новый сквад', leaderUserId: previewUserId, memberCount: 1, maxMembers: 10, imageUrl: null, isRestricted: false, restrictionReason: null, createdAt: stamp, updatedAt: stamp }
    previewSquadMembers = [{ id: previewUserId, username: nickname, avatarUrl: null, isLeader: true, inviteId: null, isPendingInvite: false }]
    return previewSquad
  }
  if (path === `/api/squads/${previewSquad?.id}/members`) return previewSquad ? previewSquadMembers.map((member) => member.id === previewUserId ? { ...member, username: nickname } : member) : []
  if (path.startsWith('/api/squads/preview-squad')) {
    if (method === 'DELETE' || path.endsWith('/leave')) { previewSquad = null; return { status: 'ok' } }
    if (method === 'PATCH') { previewSquad = { ...previewSquad, ...JSON.parse(String(init.body ?? '{}')), updatedAt: stamp }; return previewSquad }
    if (path.endsWith('/invites')) return { id: 'preview-invite', squadId: 'preview-squad', inviterUserId: previewUserId, invitedUserId: 'demo-player-2', expiresAt: stamp, createdAt: stamp, invitedUsername: 'Ворон', inviterAvatarUrl: null, inviterUsername: nickname }
    return previewSquad
  }
  if (path.startsWith('/api/squad-invites/')) return { status: 'ok' }
  if (path === '/api/users/search') return metricRows('kills').slice(0, 5).map((player) => ({ id: player.playerId, username: player.nickname, avatarUrl: null }))
  if (path.startsWith('/api/assets')) return assetList(url)
  if (path === '/api/shop/products' || path === '/api/admin/shop/products') return seed.shopProducts
  if (path === '/api/lootboxes' || path === '/api/admin/lootboxes') return seed.lootboxes
  const lootboxMatch = path.match(/^\/api\/lootboxes\/([^/]+)$/)
  if (lootboxMatch) {
    const definition = seed.lootboxes.find((item) => item.id === decodeURIComponent(lootboxMatch[1]) || item.assetKey === decodeURIComponent(lootboxMatch[1]))
    return definition ? { definition, drops: seed.lootboxDrops.filter((drop) => drop.lootboxId === definition.id) } : null
  }
  if (path === '/api/user/me') return profileUser()
  if (path === '/api/user/me/inventory/entitlements') return selectedAssets.map((asset) => ({ assetKey: asset!.key, assetDefinitionId: asset!.id, grantedAt: stamp, updatedAt: stamp }))
  if (path === '/api/user/me/inventory') return {
    userId: previewUserId,
    stackables: [],
    entitlements: selectedAssets.map((asset) => ({ assetKey: asset!.key, assetDefinitionId: asset!.id, grantedAt: stamp, updatedAt: stamp })),
    expirables: [],
  }
  if (path === '/api/user/me/wallet' || path.startsWith('/api/user/me/wallet/')) {
    if (path.endsWith('/transactions')) return [
      { id: 1, currencyAssetDefinitionId: walletAsset.id, currencyKey: walletAsset.key, operationType: 'credit', actorKind: 'system', actorUserId: null, actorServiceName: null, delta: 1500, balanceAfter: 1500, metadata: {}, reasonCode: 'welcome', reasonText: 'Стартовый баланс', createdAt: '2026-07-12T12:00:00.000Z' },
      { id: 2, currencyAssetDefinitionId: walletAsset.id, currencyKey: walletAsset.key, operationType: 'debit', actorKind: 'user', actorUserId: previewUserId, actorServiceName: null, delta: -250, balanceAfter: currencies.balance, metadata: {}, reasonCode: 'shop_purchase', reasonText: 'Покупка предмета', createdAt: stamp },
    ]
    return path.endsWith('/default') ? currencies : [currencies]
  }
  if (path === '/api/user/me/gunskins/selections') return [...selectedSkinByWeapon.entries()].map(([weaponKey, asset]) => ({ weaponKey, selected: { weaponKey, assetDefinitionId: asset.id, assetKey: asset.key, displayName: asset.displayName, description: asset.description, rarity: asset.rarity ?? 'common', selectedAt: stamp, updatedAt: stamp } }))
  const gunskinMatch = path.match(/^\/api\/user\/me\/gunskins\/([^/]+)(?:\/selected)?$/)
  if (gunskinMatch) {
    const weaponKey = decodeURIComponent(gunskinMatch[1])
    let selected = selectedSkinByWeapon.get(weaponKey)
    if (path.endsWith('/selected') && method === 'DELETE') { selectedSkinByWeapon.delete(weaponKey); return { ok: true } }
    if (path.endsWith('/selected') && method === 'PUT') {
      const requestedKey = JSON.parse(String(init.body ?? '{}')).assetKey
      selected = skinAssets.find((asset) => asset.key === requestedKey && asset.weaponKey === weaponKey) ?? selected
      if (selected) selectedSkinByWeapon.set(weaponKey, selected)
    }
    const selectedValue = selected ? { weaponKey, assetDefinitionId: selected.id, assetKey: selected.key, displayName: selected.displayName, description: selected.description, rarity: selected.rarity ?? 'common', selectedAt: stamp, updatedAt: stamp } : null
    if (path.endsWith('/selected')) return selectedValue
    return { weaponKey, selected: selectedValue, available: skinAssets.filter((asset) => asset.weaponKey === weaponKey).map((asset) => ({ id: asset.id, key: asset.key, displayName: asset.displayName, description: asset.description, weaponKey, rarity: asset.rarity ?? 'common', ownershipModel: asset.ownershipModel })) }
  }
  if (path === '/api/user/me/lootboxes') return []
  if (path === '/api/user/me/lootboxes/open-history') return []
  if (path === '/api/user/me/shop/orders') {
    if (method === 'POST') return shopOrder(JSON.parse(String(init.body ?? '{}')))
    return Object.values(shopOrders)
  }
  const orderMatch = path.match(/^\/api\/user\/me\/shop\/orders\/([^/]+)$/)
  if (orderMatch) {
    const order = shopOrders[decodeURIComponent(orderMatch[1])]
    if (path.endsWith('/receipt')) return { id: `receipt-${order?.id ?? 'preview'}`, provider: 'mock', status: 'completed', displayStatus: 'Тестовый чек', receiptUuid: null, printUrl: null, jsonUrl: null, failureProblem: null, attemptCount: 0, nextAttemptAt: null, deadlineAt: null, completedAt: stamp, createdAt: stamp, updatedAt: stamp }
    return order ?? null
  }
  if (path === '/api/metrics/leaderboard') {
    const rows = metricRows(url.searchParams.get('metric') ?? 'kills')
    const offset = Number(url.searchParams.get('offset') ?? 0)
    const limit = Number(url.searchParams.get('limit') ?? 25)
    const metric = url.searchParams.get('metric') ?? 'kills'
    const metricFields: Record<string, keyof PreviewMetricRow> = { matches_played: 'matchesPlayed', kills: 'kills', deaths: 'deaths', assists: 'assists', vehicle_destructions: 'vehicleDestructions', kd: 'kd', kda: 'kda', damage_dealt: 'damageDealt', damage_per_minute: 'damagePerMinute', win_rate: 'winRate', headshots: 'headshots', time_in_game: 'timeInGameMs' }
    const field = metricFields[metric] ?? 'kills'
    return { metric, pagination: { limit, offset, total: rows.length }, entries: rows.slice(offset, offset + limit).map((row, index) => ({ rank: offset + index + 1, metricValue: Number(row[field]), ...row })) }
  }
  const statsMatch = path.match(/^\/api\/metrics\/players\/([^/]+)\/stats$/)
  if (statsMatch) return profileStats()
  const matchesMatch = path.match(/^\/api\/metrics\/players\/([^/]+)\/matches$/)
  if (matchesMatch) {
    const all = matches()
    const offset = Number(url.searchParams.get('offset') ?? 0)
    const limit = Number(url.searchParams.get('limit') ?? 25)
    return { playerId: decodeURIComponent(matchesMatch[1]), pagination: { limit, offset, total: all.length }, matches: all.slice(offset, offset + limit) }
  }
  if (path.startsWith('/api/metrics/matches/')) {
    const gameId = decodeURIComponent(path.split('/').at(-1) ?? '')
    const details = (seed.matchDetails as Record<string, unknown>)[gameId]
    if (details) return details
    const match = matches().find((item) => item.gameId === gameId) ?? matches()[0]
    return { ...match, eventsProcessed: 0, playerCount: 0, teams: [], players: [] }
  }
  if (path === '/api/skins/me' && method === 'POST') return { status: 'success', uuid: previewUserId, model: 'default', message: 'Скин сохранён в режиме предпросмотра.' }
  if (path === '/api/auth/prepare') return { oauthUrl: withBasePath('/profile'), pow: { challenge: 'preview', difficulty: 0 } }
  if (path.startsWith('/api/admin/')) return []
  console.info(`[static preview] No fixture for ${method} ${path}`)
  return []
}
