import { useEffect, useMemo, useRef, useState } from 'react'
import './DownloadsPage.css'

type PlatformId = 'windows' | 'macos' | 'linux'

type DownloadArch = {
  id: string
  label: string
  hint: string
  href: string
}

type DownloadCardProps = {
  title: string
  description: string
  image: string
  alt: string
  recommended: boolean
  architectures: DownloadArch[]
}

type NavigatorWithUserAgentData = Navigator & {
  userAgentData?: {
    platform?: string
  }
}

function detectPlatform(): PlatformId | null {
  if (typeof navigator === 'undefined') {
    return null
  }

  const nav = navigator as NavigatorWithUserAgentData
  const platform = `${nav.userAgentData?.platform ?? navigator.platform ?? navigator.userAgent}`.toLowerCase()

  if (platform.includes('win')) {
    return 'windows'
  }

  if (platform.includes('mac')) {
    return 'macos'
  }

  if (platform.includes('linux')) {
    return 'linux'
  }

  return null
}

function DownloadCard({ title, description, image, alt, recommended, architectures }: DownloadCardProps) {
  const primaryArchitecture = architectures[0]
  const [menuOpen, setMenuOpen] = useState(false)
  const splitButtonRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    if (!menuOpen) {
      return
    }

    const onPointerDown = (event: PointerEvent) => {
      const target = event.target as Node
      if (!splitButtonRef.current?.contains(target)) {
        setMenuOpen(false)
      }
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setMenuOpen(false)
      }
    }

    window.addEventListener('pointerdown', onPointerDown)
    window.addEventListener('keydown', onKeyDown)
    return () => {
      window.removeEventListener('pointerdown', onPointerDown)
      window.removeEventListener('keydown', onKeyDown)
    }
  }, [menuOpen])

  return (
    <article className={`card downloads-card ${recommended ? 'downloads-card--recommended' : ''}`}>
      <div className="downloads-card__image-wrap">
        <img src={image} alt={alt} loading="lazy" decoding="async" width="360" height="220" />
      </div>
      <div className="downloads-card__body">
        <div className="downloads-card__head">
          <h2 className="card-title">{title}</h2>
          {recommended ? (
            <span className="ui-badge ui-badge-success">Ваша ОС</span>
          ) : (
            <span className="ui-badge ui-badge-neutral">Доступно</span>
          )}
        </div>
        <p className="card-text">{description}</p>
      </div>
      <div className="downloads-card__footer">
        <div
          className={`downloads-split-button ${architectures.length > 1 ? 'downloads-split-button--has-menu' : ''}`}
          ref={splitButtonRef}
        >
          <a
            className="btn primary downloads-split-button__main"
            href={primaryArchitecture.href}
            download
            data-analytics-event="download_launcher"
            data-cta={`downloads-${primaryArchitecture.id}`}
          >
            Скачать
            <span>{primaryArchitecture.label}</span>
          </a>
          {architectures.length > 1 ? (
            <div className="downloads-split-button__menu">
              <button
                type="button"
                aria-label={`Выбрать другую архитектуру для ${title}`}
                aria-expanded={menuOpen}
                onClick={(event) => {
                  event.preventDefault()
                  setMenuOpen((open) => !open)
                }}
              >
                ▾
              </button>
              {menuOpen ? (
                <div className="downloads-dropdown">
                  <span className="downloads-dropdown__title">Другая архитектура</span>
                  {architectures.map((architecture) => (
                    <a
                      key={architecture.id}
                      href={architecture.href}
                      download
                      data-analytics-event="download_launcher"
                      data-cta={`downloads-${architecture.id}`}
                      onClick={() => {
                        setMenuOpen(false)
                      }}
                    >
                      <span>{architecture.label}</span>
                      <small>{architecture.hint}</small>
                    </a>
                  ))}
                </div>
              ) : null}
            </div>
          ) : null}
        </div>
      </div>
    </article>
  )
}

export default function DownloadsPage() {
  const recommendedPlatform = useMemo<PlatformId>(() => detectPlatform() ?? 'windows', [])

  return (
    <main className="page downloads-page">
      <section className="downloads-hero card">
        <h1 className="card-title downloads-hero__title">Скачать лаунчер</h1>
        <p className="card-text">
          Выберите версию под вашу систему. Мы подсветили подходящую карточку автоматически, но вы можете скачать любую
          сборку вручную.
        </p>
      </section>

      <section className="downloads-grid" aria-label="Выбор операционной системы">
        <DownloadCard
          title="Windows"
          description="Установщик для Windows 10 и Windows 11. Подходит большинству игроков на ПК."
          image="/icons/windows.svg"
          alt="Логотип Windows для загрузки лаунчера SvoCraft"
          recommended={recommendedPlatform === 'windows'}
          architectures={[
            {
              id: 'win-x64',
              label: 'Windows x64',
              hint: 'Основная версия',
              href: 'https://launcher.svocraft.xyz/api/v1/file/win-x64-SvoLauncher.exe',
            }
          ]}
        />
        <DownloadCard
          title="Apple"
          description="Версия для Mac. Выберите Apple Silicon или Intel в зависимости от процессора."
          image="/icons/apple.svg"
          alt="Символ Apple для загрузки лаунчера SvoCraft на macOS"
          recommended={recommendedPlatform === 'macos'}
          architectures={[
            {
              id: 'mac-arm64',
              label: 'macOS Apple Silicon',
              hint: 'M1, M2, M3 и новее',
              href: 'https://launcher.svocraft.xyz/api/v1/file/osx-arm64-SvoLauncher',
            },
            {
              id: 'mac-x64',
              label: 'macOS Intel',
              hint: 'Intel Mac',
              href: 'https://launcher.svocraft.xyz/api/v1/file/osx-x64-SvoLauncher',
            },
          ]}
        />
        <DownloadCard
          title="Linux"
          description="Сборки для популярных Linux-дистрибутивов."
          image="/icons/linux.svg"
          alt="Символ Linux для загрузки лаунчера SvoCraft"
          recommended={recommendedPlatform === 'linux'}
          architectures={[
            {
              id: 'linux-x64',
              label: 'Linux x64',
              hint: 'Универсальная сборка',
              href: 'https://launcher.svocraft.xyz/api/v1/file/linux-x64-SvoLauncher',
            }
          ]}
        />
      </section>
    </main>
  )
}
