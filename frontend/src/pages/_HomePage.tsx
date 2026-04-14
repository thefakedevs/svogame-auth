import { useEffect, useMemo, useState, useRef, type CSSProperties } from 'react'
import { PhotoProvider, PhotoView } from 'react-photo-view'
import 'react-photo-view/dist/react-photo-view.css'
import { getMinecraftServerStatus } from '../api/meta'
import {
  buildPublicAssetImageUrl,
  listPublicAssets,
  type AssetResponse,
  type SkinRarity,
} from '../api/inventory'
import { listDiscordGuildEvents, type DiscordGuildEventResponse } from '../api/discord'
import { listPublicShopProducts, type ShopProductResponse } from '../api/shop'
import FooterSection from './landing/sections/FooterSection'
import './HomePage.css'
import './OwnershipPage.css'

type OnlineCardState =
  | { status: 'loading' }
  | { status: 'ready'; online: boolean; playersOnline: number; playersMax: number | null }
  | { status: 'error' }

function formatPlayers(count: number): string {
  const normalized = Math.abs(count)
  const lastTwo = normalized % 100
  const last = normalized % 10

  if (last === 1 && lastTwo !== 11) {
    return `${count} игрок`
  }

  if (last >= 2 && last <= 4 && (lastTwo < 12 || lastTwo > 14)) {
    return `${count} игрока`
  }

  return `${count} игроков`
}

function getOnlineCardCopy(state: OnlineCardState): {
  badgeClassName: string
  badgeText: string
  headline: string
  text: string
} {
  if (state.status === 'loading') {
    return {
      badgeClassName: 'ui-badge ui-badge-neutral',
      badgeText: 'Проверяем онлайн',
      headline: 'Загрузка',
      text: 'Получаем статус сервера',
    }
  }

  if (state.status === 'error') {
    return {
      badgeClassName: 'ui-badge ui-badge-warning',
      badgeText: 'Статус недоступен',
      headline: 'Нет данных',
      text: 'Не удалось получить онлайн сервера',
    }
  }

  if (!state.online) {
    return {
      badgeClassName: 'ui-badge ui-badge-warning',
      badgeText: 'Сервер офлайн',
      headline: formatPlayers(0),
      text: 'Сейчас сервер не отвечает на проверку статуса.',
    }
  }

  return {
    badgeClassName: 'ui-badge ui-badge-success',
    badgeText: 'Сейчас онлайн',
    headline: formatPlayers(state.playersOnline),
    text: 'Онлайн сервера обновляется автоматически.',
  }
}

type DiscordEventsState =
  | { status: 'loading' }
  | { status: 'ready'; events: DiscordGuildEventResponse[] }
  | { status: 'error' }

function isDiscordGameLive(events: DiscordGuildEventResponse[]): boolean {
  return events.some((e) => e.status === 'active')
}

function getGameLiveBadgeCopy(state: DiscordEventsState): { className: string; text: string } {
  if (state.status === 'loading') {
    return { className: 'ui-badge ui-badge-neutral', text: 'Ивент: загрузка' }
  }

  if (state.status === 'error') {
    return { className: 'ui-badge ui-badge-warning', text: 'Ивент: нет данных' }
  }

  if (isDiscordGameLive(state.events)) {
    return { className: 'ui-badge ui-badge-success', text: 'Игра идёт' }
  }

  return { className: 'ui-badge ui-badge-danger', text: 'Игра не идёт' }
}

const landingEventDateFormatter = new Intl.DateTimeFormat('ru-RU', {
  day: 'numeric',
  month: 'short',
  hour: '2-digit',
  minute: '2-digit',
})

function discordEventStatusLabel(status: string): string {
  switch (status) {
    case 'active':
      return 'Сейчас'
    case 'scheduled':
      return 'Скоро'
    case 'completed':
      return 'Завершено'
    case 'canceled':
      return 'Отменено'
    default:
      return 'Ивент'
  }
}

function pickLandingScheduleEvents(events: DiscordGuildEventResponse[]): DiscordGuildEventResponse[] {
  const now = Date.now()

  return events
    .filter((e) => {
      if (e.status === 'completed' || e.status === 'canceled') return false
      if (e.status === 'active') return true
      const end = e.endsAt ? new Date(e.endsAt).getTime() : null
      if (e.status === 'scheduled' && end !== null && end < now) return false

      return true
    })
    .sort((a, b) => {
      const priority = (ev: DiscordGuildEventResponse) => (ev.status === 'active' ? 0 : 1)
      const order = priority(a) - priority(b)
      if (order !== 0) return order

      return new Date(a.startsAt).getTime() - new Date(b.startsAt).getTime()
    })
    .slice(0, 10)
}

type ShopTickerState =
  | { status: 'loading' }
  | { status: 'ready'; products: ShopProductResponse[]; assets: AssetResponse[] }
  | { status: 'error' }

const skinRarityConfig: Record<SkinRarity, { label: string; color: string }> = {
  common: { label: 'Обычный', color: '#9aa8b5' },
  rare: { label: 'Редкий', color: '#2f8cff' },
  legendary: { label: 'Легендарный', color: '#ffb02e' },
}

const priceFormatter = new Intl.NumberFormat('ru-RU', {
  style: 'currency',
  currency: 'RUB',
  maximumFractionDigits: 0,
})

function formatPrice(value: number) {
  return priceFormatter.format(value)
}

function cardStyle(accent: string): CSSProperties {
  return { '--inventory-accent': accent } as CSSProperties
}

function metadataRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === 'object' && !Array.isArray(value) ? value as Record<string, unknown> : null
}

function metadataString(source: { metadata?: unknown } | null, keys: string[]) {
  const metadata = metadataRecord(source?.metadata)
  if (!metadata) return null

  for (const key of keys) {
    const value = metadata[key]
    if (typeof value === 'string' && value.trim()) return value.trim()
  }

  return null
}

function assetImageUrl(asset: AssetResponse | null) {
  if (!asset) return null
  return asset.imageUrl
    ?? metadataString(asset, ['imageUrl', 'image', 'iconUrl', 'icon', 'thumbnailUrl', 'thumbnail', 'previewUrl', 'skinUrl'])
    ?? buildPublicAssetImageUrl(asset.id, asset.updatedAt)
}

function fallbackAccent(key: string) {
  const palette = ['#2db7a3', '#f7a41d', '#67d391', '#ff8b8b', '#6fb6ff', '#d6a4ff']
  let hash = 0
  for (const char of key) hash = (hash + char.charCodeAt(0)) % palette.length
  return palette[hash]
}

function productAccent(product: ShopProductResponse, asset: AssetResponse | null) {
  return asset?.rarity
    ? skinRarityConfig[asset.rarity].color
    : metadataString(asset, ['accentColor', 'color', 'rarityColor'])
      ?? metadataString(product, ['accentColor', 'color', 'rarityColor'])
      ?? fallbackAccent(product.assetKey)
}

function rarityLabel(rarity: SkinRarity | null | undefined) {
  return rarity ? skinRarityConfig[rarity].label : 'Скин'
}

function buildAssetMap(assets: AssetResponse[]) {
  const map = new Map<string, AssetResponse>()
  for (const asset of assets) {
    map.set(asset.key, asset)
    map.set(asset.id, asset)
  }
  return map
}

function productDescription(product: ShopProductResponse, asset: AssetResponse | null) {
  return product.localizedDescription
    ?? asset?.description
    ?? metadataString(product, ['description', 'details'])
    ?? metadataString(asset, ['description', 'details'])
    ?? 'Скин для твоего инвентаря.'
}

function isSkinProduct(product: ShopProductResponse, asset: AssetResponse | null) {
  if (product.ownershipModel !== 'entitlement') return false
  if (!asset) return true
  if (asset.assetKind === 'skin' || asset.weaponKey) return true

  const haystack = [
    product.key,
    product.assetKey,
    product.localizedName,
    product.assetDisplayName,
    asset.key,
    asset.displayName,
    asset.assetKind,
  ].join(' ').toLowerCase()

  return haystack.includes('skin') || haystack.includes('скин')
}

async function listAllEntitlementAssets(perPage = 100) {
  const items: AssetResponse[] = []
  let page = 1

  while (true) {
    const response = await listPublicAssets({ ownershipModel: 'entitlement', page, perPage })
    items.push(...response.items)

    if (items.length >= response.total || response.items.length === 0) {
      return items
    }

    page += 1
  }
}

type ShopProductView = {
  product: ShopProductResponse
  asset: AssetResponse | null
  title: string
  description: string
  imageUrl: string | null
  accent: string
}

function InventoryVisual({ item }: { item: { title: string; imageUrl: string | null; accent: string } }) {
  const [failedImageUrl, setFailedImageUrl] = useState<string | null>(null)
  const fallback = item.title.trim().slice(0, 1).toUpperCase() || 'S'
  const showImage = Boolean(item.imageUrl) && failedImageUrl !== item.imageUrl

  return (
    <div className="inventory-visual" style={cardStyle(item.accent)}>
      {showImage ? <img src={item.imageUrl ?? ''} alt="" loading="lazy" draggable={false} onError={() => setFailedImageUrl(item.imageUrl)} /> : <span>{fallback}</span>}
      <div className="inventory-visual-splash" />
    </div>
  )
}

function ShopTickerCard({ item, isDuplicate }: { item: ShopProductView; isDuplicate?: boolean }) {
  return (
    <article
      className="landing-shop-ticker-card"
      data-rarity={item.asset?.rarity ?? 'none'}
      style={cardStyle(item.accent)}
      aria-hidden={isDuplicate}
    >
      <div className="landing-shop-ticker-card__visual">
        <div className="landing-shop-ticker-card__overlay">
          <strong className="landing-shop-ticker-card__title">{item.title}</strong>
          <span className="landing-shop-ticker-card__rarity">{rarityLabel(item.asset?.rarity)}</span>
        </div>
        <InventoryVisual item={item} />
      </div>
      <div className="landing-shop-ticker-card__footer">
        <span className="landing-shop-ticker-card__price">{formatPrice(item.product.priceRub)}</span>
      </div>
    </article>
  )
}

export default function HomePage() {
  const [onlineState, setOnlineState] = useState<OnlineCardState>({ status: 'loading' })
  const [discordEventsState, setDiscordEventsState] = useState<DiscordEventsState>({ status: 'loading' })
  const [shopState, setShopState] = useState<ShopTickerState>({ status: 'loading' })
  const onlineCopy = getOnlineCardCopy(onlineState)
  const gameLiveCopy = getGameLiveBadgeCopy(discordEventsState)
  const scheduleEvents = discordEventsState.status === 'ready'
    ? pickLandingScheduleEvents(discordEventsState.events)
    : []

  const productViews = useMemo(() => {
    if (shopState.status !== 'ready') return []

    const assetMap = buildAssetMap(shopState.assets)

    return shopState.products
      .map((product) => {
        const asset = assetMap.get(product.assetKey) ?? assetMap.get(product.assetDefinitionId) ?? null
        const title = product.localizedName || asset?.displayName || product.assetDisplayName || product.key
        const description = productDescription(product, asset)

        return {
          product,
          asset,
          title,
          description,
          imageUrl: assetImageUrl(asset),
          accent: productAccent(product, asset),
        }
      })
      .filter((item) => isSkinProduct(item.product, item.asset))
      .sort((left, right) => left.product.sortOrder - right.product.sortOrder || left.title.localeCompare(right.title, 'ru'))
      .slice(0, 8) // Limit to 8 items for ticker
  }, [shopState])

const ref = useRef<HTMLDivElement>(null)

useEffect(() => {
  const el = ref.current
  if (!el) return

  let frame = 0
  let pos = 0

  const baseSpeed = 1
  let autoSpeed = baseSpeed
  let velocity = 0

  let isHovering = false
  let isDragging = false

  let pointerId: number | null = null
  let lastClientX = 0
  let lastMoveTime = 0

  const normalize = () => {
    const halfWidth = el.scrollWidth / 2
    if (halfWidth <= 0) return

    while (pos <= -halfWidth) pos += halfWidth
    while (pos > 0) pos -= halfWidth
  }

  const render = () => {
    el.style.setProperty('--ticker-x', `${pos}px`)
  }

  const tick = () => {
    if (!isDragging) {
      if (Math.abs(velocity) > 0.01) {
        pos += velocity
        velocity *= 0.95
      } else {
        velocity = 0
        const targetSpeed = isHovering ? 0 : baseSpeed
        autoSpeed += (targetSpeed - autoSpeed) * 0.035
        pos -= autoSpeed
      }

      normalize()
      render()
    }

    frame = requestAnimationFrame(tick)
  }

  const onPointerEnter = () => {
    isHovering = true
  }

  const onPointerLeave = () => {
    isHovering = false
  }

  const onPointerDown = (event: PointerEvent) => {
    isDragging = true
    pointerId = event.pointerId
    lastClientX = event.clientX
    lastMoveTime = performance.now()
    velocity = 0
    autoSpeed = 0

    el.setPointerCapture(event.pointerId)
    el.style.cursor = 'grabbing'
    el.style.userSelect = 'none'
  }

  const onPointerMove = (event: PointerEvent) => {
    if (!isDragging || pointerId !== event.pointerId) return

    const now = performance.now()
    const dx = event.clientX - lastClientX
    const dt = Math.max(now - lastMoveTime, 1)

    pos += dx
    normalize()
    render()

    velocity = dx / (dt / 16.6667)
    lastClientX = event.clientX
    lastMoveTime = now
  }

  const finishDrag = (event?: PointerEvent) => {
    if (!isDragging) return
    if (event && pointerId !== event.pointerId) return

    isDragging = false
    pointerId = null
    el.style.cursor = ''
    el.style.userSelect = ''
  }

  const onPointerUp = (event: PointerEvent) => {
    finishDrag(event)
  }

  const onPointerCancel = (event: PointerEvent) => {
    finishDrag(event)
  }

  el.addEventListener('pointerenter', onPointerEnter)
  el.addEventListener('pointerleave', onPointerLeave)
  el.addEventListener('pointerdown', onPointerDown)
  el.addEventListener('pointermove', onPointerMove)
  el.addEventListener('pointerup', onPointerUp)
  el.addEventListener('pointercancel', onPointerCancel)

  render()
  frame = requestAnimationFrame(tick)

  return () => {
    cancelAnimationFrame(frame)
    el.removeEventListener('pointerenter', onPointerEnter)
    el.removeEventListener('pointerleave', onPointerLeave)
    el.removeEventListener('pointerdown', onPointerDown)
    el.removeEventListener('pointermove', onPointerMove)
    el.removeEventListener('pointerup', onPointerUp)
    el.removeEventListener('pointercancel', onPointerCancel)
  }
}, [productViews.length])

  useEffect(() => {
    let isMounted = true

    getMinecraftServerStatus()
      .then((status) => {
        if (!isMounted) return
        setOnlineState({
          status: 'ready',
          online: status.online,
          playersOnline: status.playersOnline,
          playersMax: status.playersMax,
        })
      })
      .catch(() => {
        if (!isMounted) return
        setOnlineState({ status: 'error' })
      })

    Promise.all([
      listPublicShopProducts('ru-RU'),
      listAllEntitlementAssets(),
    ])
      .then(([products, assets]) => {
        if (!isMounted) return
        setShopState({ status: 'ready', products, assets })
      })
      .catch(() => {
        if (!isMounted) return
        setShopState({ status: 'error' })
      })

    const loadDiscordEvents = () => {
      listDiscordGuildEvents()
        .then((events) => {
          if (!isMounted) return
          setDiscordEventsState({ status: 'ready', events })
        })
        .catch(() => {
          if (!isMounted) return
          setDiscordEventsState({ status: 'error' })
        })
    }

    loadDiscordEvents()
    const discordPoll = window.setInterval(loadDiscordEvents, 45_000)

    return () => {
      isMounted = false
      window.clearInterval(discordPoll)
    }
  }, [])

  return (
    <main className="page home-page landing-page">
      <section className="landing-hero" aria-labelledby="landing-hero-title">
        <img
          className="landing-hero__image"
          src="/landing/hero.jpg"
          alt=""
          fetchPriority="high"
          aria-hidden="true"
        />
        <div className="landing-hero__shade" aria-hidden />
        <div className="landing-hero__content">
          <h1 id="landing-hero-title" className="landing-hero__title">Зайди в бой за две минуты</h1>
          <p className="landing-hero__lead">
            Командные ивенты, быстрые мини-игры и вход без лишней настройки. Скачайте лаунчер, авторизуйтесь через Discord
            и заходите в матч, пока арены горячие.
          </p>
          <div className="landing-hero__actions">
            <a
              className="btn primary btn-lg"
              href="/auth"
              data-analytics-event="start_play"
              data-cta="hero-start-play"
            >
              Начать играть
            </a>
            <a
              className="btn btn-secondary-accent btn-lg"
              href="/downloads"
              data-analytics-event="download_launcher"
              data-cta="hero-download-launcher"
              aria-label="Скачать лаунчер SvoCraft"
            >
              Скачать лаунчер
            </a>
          </div>
          <ul className="landing-trust" aria-label="Почему можно начинать сразу">
            <li>Безопасная авторизация через Discord</li>
            <li>Активное сообщество</li>
            <li>Быстрый вход без ручной настройки</li>
          </ul>
        </div>
      </section>

      <section className="landing-section flex flex-col gap-4" aria-labelledby="landing-modes-title">
        <div className="landing-section-head">
          <span className="ui-section-title">Режимы</span>
          <h2 id="landing-modes-title" className="landing-section-head__title">Матчи под разный темп</h2>
        </div>
        <div className="landing-mode-grid">
          <article className="card landing-mode-card">
            <div className="landing-card-topline">
              <h3 className="card-title">Ивенты</h3>
              <span className="ui-badge ui-badge-accent">2 раза в неделю</span>
            </div>
            <p className="card-text">
              Крупные события с участием большого количества игроков, активной модерацией и приработанными системами.
            Является приоритетным и самым ожидаемым режимом среди игроков.
            </p>
            <div className="ui-card-footer">
              <a
                className="btn btn-sm btn-secondary-accent"
                href="/auth"
                data-analytics-event="start_play"
                data-cta="mode-capture-flag-start"
              >
                Подготовиться к ивенту
              </a>
            </div>
          </article>

          <article className="card landing-mode-card">
            <div className="landing-card-topline">
              <h3 className="card-title">Мини-игры</h3>
              <span className="ui-badge ui-badge-success">Доступны всегда</span>
            </div>
            <p className="card-text">
              Небольшие режимы доступные абсолютно всегда. Отлично подходят для игры в свободное время с своими друзьями, а также для знакомства с механиками и оттачивания навыков.
            </p>
            <div className="ui-card-footer">
              <a
                className="btn btn-sm btn-secondary-accent"
                href="/auth"
                data-analytics-event="start_play"
                data-cta="mode-duels-start"
              >
                Войти в мини-игру
              </a>
            </div>
          </article>
        </div>
      </section>

      <section className="landing-section landing-start" aria-labelledby="landing-start-title">
        <div className="landing-section-head">
          <span className="ui-section-title">Как начать</span>
          <h2 id="landing-start-title" className="landing-section-head__title">Четыре шага до первого матча</h2>
          <p className="landing-section-head__text">
            Лаунчер сам держит клиент в актуальном состоянии, а вход через Discord привязывает профиль к аккаунту
            SvoCraft.
          </p>
        </div>
        <div className="landing-steps" aria-label="Шаги запуска">
          <div className="landing-step">
            <span className="landing-step__index">1</span>
            <span>Скачайте лаунчер</span>
          </div>
          <div className="landing-step">
            <span className="landing-step__index">2</span>
            <span>Перенесите его на рабочий стол</span>
          </div>
          <div className="landing-step">
            <span className="landing-step__index">3</span>
            <span>Войдите или зарегистрируйтесь через Discord</span>
          </div>
          <div className="landing-step">
            <span className="landing-step__index">4</span>
            <span>Нажмите «Играть» и заходите на сервер</span>
          </div>
          <div className="landing-start__actions">
            <a
              className="btn primary btn-lg"
              href="/downloads"
              data-analytics-event="download_launcher"
              data-cta="start-download-launcher"
            >
              Скачать лаунчер
            </a>
          </div>
        </div>
      </section>

      <section id="gallery" className="landing-section" aria-labelledby="landing-gallery-title">
        <div className="landing-section-head">
          <span className="ui-section-title">Галерея</span>
          <h2 id="landing-gallery-title" className="landing-section-head__title">Галерея</h2>
          <p className="landing-section-head__text">
            Превью арен, мини-игр и лаунчера. Нажмите на кадр, чтобы открыть изображение на весь экран.
          </p>
        </div>
        <PhotoProvider loop photoClosable maskOpacity={0.92}>
          <div className="landing-gallery-grid">
            <PhotoView
              src="/landing/2026-04-12_20.33.02.png"
              width={1280}
              height={720}
              overlay={(
                <div className="landing-photo-overlay">
                  <strong>Одна из ивентовых карт</strong>
                  <span>Карта для событий с упором на манёвры, спроектированная с вниманием к деталям, специально под режим, который на ней проводится.</span>
                </div>
              )}
            >
              <button
                type="button"
                className="landing-gallery-card"
                data-analytics-event="open_gallery"
                data-cta="gallery-capture-flag-arena"
                aria-label="Открыть изображение: Арена захвата флага"
              >
                <img
                  src="/landing/2026-04-12_20.33.02.png"
                  width={1280}
                  height={720}
                  loading="lazy"
                  decoding="async"
                />
                <span className="landing-gallery-card__caption">
                  <span>Одна из ивентовых карт</span>
                  <small>Карта для событий с упором на манёвры, спроектированная с вниманием к деталям, под режим, который на ней проводится.</small>
                </span>
              </button>
            </PhotoView>

            <PhotoView
              src="/landing/2026-04-12_22.13.53.png"
              width={1280}
              height={720}
              overlay={(
                <div className="landing-photo-overlay">
                  <strong>Дуэльная линия</strong>
                  <span>Минималистичная арена, где исход решают точность, тайминг и хладнокровие, разработанная специально для того, чтобы позволить показать свой скилл.</span>
                </div>
              )}
            >
              <button
                type="button"
                className="landing-gallery-card"
                data-analytics-event="open_gallery"
                data-cta="gallery-duel-bridge"
                aria-label="Открыть изображение: Дуэльная линия"
              >
                <img
                  src="/landing/2026-04-12_22.13.53.png"
                  width={1280}
                  height={720}
                  loading="lazy"
                  decoding="async"
                />
                <span className="landing-gallery-card__caption">
                  <span>Дуэльная линия</span>
                  <small>Минималистичная арена, где исход решают точность, тайминг и хладнокровие, разработанная специально для того, чтобы позволить показать свой скилл.</small>
                </span>
              </button>
            </PhotoView>

            <PhotoView
              src="/landing/2026-04-12_21.50.19.png"
              width={1280}
              height={720}
              overlay={(
                <div className="landing-photo-overlay">
                  <strong>Лобби</strong>
                  <span>Безопасная точка старта, где игроки собирают отряд перед матчем, формируют отряды, готовятся к бою и выбирают дальнейший путь.</span>
                </div>
              )}
            >
              <button
                type="button"
                className="landing-gallery-card"
                data-analytics-event="open_gallery"
                data-cta="gallery-team-spawn"
                aria-label="Открыть изображение: Командный спавн"
              >
                <img
                  src="/landing/2026-04-12_21.50.19.png"
                  width={1280}
                  height={720}
                  loading="lazy"
                  decoding="async"
                />
                <span className="landing-gallery-card__caption">
                  <span>Лобби</span>
                  <small>Безопасная точка старта, где игроки собирают отряд перед матчем, формируют отряды, готовятся к бою и выбирают дальнейший путь.</small>
                </span>
              </button>
            </PhotoView>

            <PhotoView
              src="/landing/launcher.png"
              width={1280}
              height={720}
              overlay={(
                <div className="landing-photo-overlay">
                  <strong>Лаунчер</strong>
                  <span>Современный центр управления игрой, через который осуществляется вход, загрузка обновлений и быстрый запуск клиента без лишних действий.</span>
                </div>
              )}
            >
              <button
                type="button"
                className="landing-gallery-card"
                data-analytics-event="open_gallery"
                data-cta="gallery-launcher"
                aria-label="Открыть изображение: Лаунчер"
              >
                <img
                  src="/landing/launcher.png"
                  width={1280}
                  height={720}
                  loading="lazy"
                  decoding="async"
                />
                <span className="landing-gallery-card__caption">
                  <span>Лаунчер</span>
                  <small>Современный центр управления игрой, через который осуществляется вход, загрузка обновлений и быстрый запуск клиента без лишних действий.</small>
                </span>
              </button>
            </PhotoView>
          </div>
        </PhotoProvider>
      </section>

      <section id="shop" className="landing-section" aria-labelledby="landing-shop-title">
        <div className="landing-section-head">
          <span className="ui-section-title">Магазин</span>
          <h2 id="landing-shop-title" className="landing-section-head__title">Покупайте косметику и поддерживайте сервер</h2>
          <p className="landing-section-head__text">
            В нашем магазине вы найдете эксклюзивные косметические предметы, которые не только украсят ваш игровой опыт, но и помогут поддержать развитие сервера.
          </p>
        </div>
        <div className="landing-shop-grid">
          {shopState.status === 'loading' && (
            <div className="landing-shop-loading">
              <span>Загрузка товаров магазина...</span>
            </div>
          )}
          {shopState.status === 'error' && (
            <div className="landing-shop-error">
              <span>Не удалось загрузить товары магазина.</span>
            </div>
          )}
          {shopState.status === 'ready' && productViews.length === 0 && (
            <div className="landing-shop-empty">
              <span>Ассортимент магазина скоро появится.</span>
            </div>
          )}
          {shopState.status === 'ready' && productViews.length > 0 && (
            <div className="landing-shop-ticker" aria-live="polite" aria-label="Популярные товары магазина">
              <div ref={ref} className="landing-shop-ticker__track">
                {productViews.map((item) => (
                  <ShopTickerCard
                    key={`original-${item.product.id}`}
                    item={item}
                  />
                ))}

                {productViews.map((item) => (
                  <ShopTickerCard
                    key={`duplicate-${item.product.id}`}
                    item={item}
                    isDuplicate
                  />
                ))}
              </div>
            </div>
          )}
        </div>
        <div className="landing-ticker__actions">
          <a
            className="btn btn-secondary-accent"
            href="/shop"
            data-analytics-event="shop_view_all"
            data-cta="shop-ticker-view-all"
          >
            Перейти в магазин
          </a>
        </div>
      </section>

      <section id="community" className="landing-section" aria-labelledby="landing-community-title">
        <div className="landing-section-head">
          <span className="ui-section-title">Сообщество</span>
          <h2 id="landing-community-title" className="landing-section-head__title">Не заходите в пустоту</h2>
          <p className="landing-section-head__text">
            Игроки держат темп в мини-играх, собираются на ивенты и помогают новичкам быстрее попасть в первый матч.
          </p>
        </div>
        <div className="landing-community-grid">
          <div className="landing-community-hero">
            <div className="card landing-online-card" aria-live="polite">
              <div className="landing-online-card__badges">
                <span className={onlineCopy.badgeClassName}>{onlineCopy.badgeText}</span>
                <span className={gameLiveCopy.className}>{gameLiveCopy.text}</span>
              </div>
              <strong>{onlineCopy.headline}</strong>
              <p className="card-text">{onlineCopy.text}</p>
              <div className="landing-community-actions">
                <a
                  className="btn primary btn-sm"
                  href="https://discord.gg/UQQK4ykxMa"
                  data-analytics-event="join_discord"
                  data-cta="community-discord"
                  target="_blank"
                  rel="noreferrer"
                >
                  В Discord
                </a>
                <a
                  className="btn btn-sm"
                  href="https://t.me/sv0craft"
                  data-cta="community-telegram"
                  target="_blank"
                  rel="noreferrer"
                >
                  В Telegram
                </a>
              </div>
            </div>
            <div className="card landing-events-card" aria-live="polite" aria-label="Расписание игр из Discord">
              <div className="landing-events-card__head">
                <h3 className="landing-events-card__title">Расписание игр</h3>
              </div>
              {discordEventsState.status === 'loading' && (
                <p className="card-text landing-events-card__muted">Загружаем ближайшие ивенты…</p>
              )}
              {discordEventsState.status === 'error' && (
                <p className="card-text landing-events-card__muted">
                  Не удалось загрузить расписание. Загляните в Discord — там всегда актуально.
                </p>
              )}
              {discordEventsState.status === 'ready' && scheduleEvents.length === 0 && (
                <p className="card-text landing-events-card__muted">
                  Пока нет запланированных ивентов. Следите за анонсами в Discord.
                </p>
              )}
              {discordEventsState.status === 'ready' && scheduleEvents.length > 0 && (
                <ul className="landing-events-card__list">
                  {scheduleEvents.map((ev) => (
                    <li key={ev.id} className="landing-events-card__row">
                      <div className="landing-events-card__meta">
                        <time dateTime={ev.startsAt}>{landingEventDateFormatter.format(new Date(ev.startsAt))}</time>
                        <span
                          className={
                            ev.status === 'active'
                              ? 'ui-badge ui-badge-success'
                              : 'ui-badge ui-badge-neutral'
                          }
                        >
                          {discordEventStatusLabel(ev.status)}
                        </span>
                      </div>
                      <div className="landing-events-card__body">
                        <span className="landing-events-card__name">{ev.name}</span>
                        {typeof ev.userCount === 'number' && ev.userCount > 0 ? (
                          <span className="landing-events-card__count">
                            {ev.userCount}
                            {' '}
                            участ.
                          </span>
                        ) : null}
                      </div>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </div>
          <div className="landing-testimonials">
            <blockquote className="card landing-testimonial">
              <p>
                Зашёл на сервер, сначала было немного непривычно, но быстро втянулся, лёг на балконе,
                взял монитор в руки и начал управление дроном, пару дронов, и вражеская пума подбита,
                на вырученные средства прикупил себе новые зимние ботинки, теперь питаюсь только стейками,
                прикупил себе новенький AWM, которым отстреливаю различную перхоть на своей, немного побитой
                терассе, попивая чашечку кофе.
              </p>
              <footer>
                <strong>Eblan</strong>
                <span>Ветеран сервера</span>
              </footer>
            </blockquote>
            <blockquote className="card landing-testimonial">
              <p>
                SvoCraft — Отличный сервер для тех, кто любит PvP. Захват флага держит в напряжении до
                последней секунды, а дуэли — лучший способ проверить свою реакцию. Заходите, не пожалеете.
              </p>
              <footer>
                <strong>Krevetka42</strong>
                <span>Восхитительный боец</span>
              </footer>
            </blockquote>
          </div>
        </div>
      </section>

      <section className="landing-final-cta" aria-labelledby="landing-final-title">
        <div className="landing-final-cta__content">
          <span className="ui-badge ui-badge-secondary">Готовы к матчу</span>
          <h2 id="landing-final-title">Играйте с нами!</h2>
          <p>
            Постоянные мини-игры держат форму, а ивентовые игры дважды в неделю дает цель для сквада. Чем раньше зайдете, тем
            быстрее привыкнете к аренам и таймингам.
          </p>
        </div>
        <div className="landing-final-cta__actions">
          <a
            className="btn primary btn-lg"
            href="/auth"
            data-analytics-event="start_play"
            data-cta="final-start-play"
          >
            Играть у нас
          </a>
          <a
            className="btn btn-lg"
            href="/downloads"
            data-analytics-event="download_launcher"
            data-cta="final-download-launcher"
          >
            Скачать лаунчер
          </a>
        </div>
      </section>

      <FooterSection />
    </main>
  )
}
