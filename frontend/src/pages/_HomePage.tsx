import { useEffect, useState } from 'react'
import { PhotoProvider, PhotoView } from 'react-photo-view'
import 'react-photo-view/dist/react-photo-view.css'
import { getMinecraftServerStatus } from '../api/meta'
import FooterSection from './landing/sections/FooterSection'
import './HomePage.css'

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

export default function HomePage() {
  const [onlineState, setOnlineState] = useState<OnlineCardState>({ status: 'loading' })
  const onlineCopy = getOnlineCardCopy(onlineState)

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

    return () => {
      isMounted = false
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
              aria-label="Скачать лаунчер SVOCraft"
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
            SVOCraft.
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

      <section id="community" className="landing-section" aria-labelledby="landing-community-title">
        <div className="landing-section-head">
          <span className="ui-section-title">Сообщество</span>
          <h2 id="landing-community-title" className="landing-section-head__title">Не заходите в пустоту</h2>
          <p className="landing-section-head__text">
            Игроки держат темп в мини-играх, собираются на ивенты и помогают новичкам быстрее попасть в первый матч.
          </p>
        </div>
        <div className="landing-community-grid">
          <div className="card landing-online-card" aria-live="polite">
            <span className={onlineCopy.badgeClassName}>{onlineCopy.badgeText}</span>
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
                SVOcraft — Отличный сервер для тех, кто любит PvP. Захват флага держит в напряжении до
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
