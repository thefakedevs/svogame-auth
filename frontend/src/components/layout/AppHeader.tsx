import { useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { toDisplayError } from '../../api/http'
import { buildAuthUrl } from '../../routes/auth'
import { paths } from '../../routes/paths'
import { tokenManager } from '../../services/tokenManager'
import { useAuthStore } from '../../store/authStore'
import './AppHeader.css'

type Props = {
  pageTitle?: string
}

export default function AppHeader({ pageTitle }: Props) {
  const authHydrated = useAuthStore((store) => store.hydrated)
  const authUser = useAuthStore((store) => store.user)
  const setAuthUser = useAuthStore((store) => store.setUser)
  const setAuthToken = useAuthStore((store) => store.setToken)

  const [isMenuOpen, setIsMenuOpen] = useState(false)
  const [sessionWarning, setSessionWarning] = useState<string | null>(null)
  const triggerRef = useRef<HTMLDivElement | null>(null)
  const menuPanelRef = useRef<HTMLDivElement | null>(null)

  const portalTarget =
    typeof document !== 'undefined' ? document.querySelector('.ui-kit-page.app-shell') ?? document.body : null

  useEffect(() => {
    const onPointerDown = (event: PointerEvent) => {
      const target = event.target as Node
      const clickedTrigger = triggerRef.current?.contains(target)
      const clickedMenu = menuPanelRef.current?.contains(target)

      if (!clickedTrigger && !clickedMenu) {
        setIsMenuOpen(false)
      }
    }

    window.addEventListener('pointerdown', onPointerDown)
    return () => window.removeEventListener('pointerdown', onPointerDown)
  }, [])

  useEffect(() => {
    if (!isMenuOpen) return

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setIsMenuOpen(false)
      }
    }

    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [isMenuOpen])

  useEffect(() => {
    if (!isMenuOpen) return

    const previousOverflow = document.body.style.overflow
    document.body.style.overflow = 'hidden'

    return () => {
      document.body.style.overflow = previousOverflow
    }
  }, [isMenuOpen])

  useEffect(() => {
    async function validateSession() {
      const token = await tokenManager.getToken()
      if (!token) {
        setSessionWarning(null)
        return
      }

      if (!tokenManager.validateTokenFormat(token)) {
        setSessionWarning('Сохраненная сессия выглядит поврежденной. Лучше войти заново.')
        return
      }

      setSessionWarning(null)
    }

    void validateSession()
  }, [authHydrated])

  const avatarFallback = authUser?.username ? authUser.username.slice(0, 2).toUpperCase() : 'SV'
  const hasSession = authHydrated && Boolean(authUser)

  const onLogout = async () => {
    try {
      await tokenManager.clearToken()
      setAuthToken(null)
      setAuthUser(null)
      window.location.assign(paths.home)
    } catch (error) {
      setSessionWarning(toDisplayError(error, 'Не удалось завершить сессию на этом устройстве.'))
    }
  }

  return (
    <header className="app-header">
      <div className="app-header__left">
        <a href={paths.home} className="app-header__brand">
          SVOCraft
        </a>
      </div>

      <div className="app-header__center" aria-live="polite">
        {pageTitle ? <span className="app-header__title">{pageTitle}</span> : null}
      </div>

      <div className="app-header__right" ref={triggerRef}>
        {!authHydrated ? (
          <span className="app-header__auth-placeholder" aria-hidden />
        ) : hasSession ? (
          <>
            <button
              className="app-header__account-trigger"
              type="button"
              onClick={() => setIsMenuOpen((open) => !open)}
              aria-expanded={isMenuOpen}
              aria-controls="app-header-menu"
            >
              {authUser?.avatarUrl ? (
                <img src={authUser.avatarUrl} alt={authUser.username} className="app-header__avatar" />
              ) : (
                <span className="app-header__avatar app-header__avatar--fallback">{avatarFallback}</span>
              )}
              <span className="app-header__account-name">{authUser?.username ?? 'Игрок'}</span>
              <span className="app-header__account-caret" aria-hidden>
                ▾
              </span>
            </button>

            {isMenuOpen && portalTarget
              ? createPortal(
                  <div
                    id="app-header-menu"
                    ref={menuPanelRef}
                    className="app-header__menu"
                    role="dialog"
                    aria-modal="true"
                    aria-label="Навигация"
                  >
                    <div className="app-header__menu-topbar">
                      <span className="app-header__menu-kicker">Меню</span>
                      <button
                        className="app-header__menu-close"
                        type="button"
                        aria-label="Закрыть меню"
                        onClick={() => setIsMenuOpen(false)}
                      >
                        ×
                      </button>
                    </div>

                    <div className="app-header__menu-body">
                      <div className="app-header__menu-head">
                        {authUser?.avatarUrl ? (
                          <img src={authUser.avatarUrl} alt={authUser.username} className="app-header__menu-avatar" />
                        ) : (
                          <span className="app-header__menu-avatar app-header__avatar--fallback">{avatarFallback}</span>
                        )}
                        <div>
                          <strong>{authUser?.username ?? 'Игрок'}</strong>
                          <div className="app-header__menu-meta">Discord подключен</div>
                        </div>
                      </div>

                      {sessionWarning ? (
                        <div className="ui-alert ui-alert-warning">
                          <span className="ui-alert-icon" aria-hidden>
                            !
                          </span>
                          <span>{sessionWarning}</span>
                        </div>
                      ) : null}

                      <div className="app-header__menu-section">
                        <a href={paths.profile}>Профиль</a>
                        <a href={`${paths.profile}?tab=squads`}>Сквад</a>
                        <a href={`${paths.profile}?tab=settings`}>Настройки</a>
                      </div>

                      <div className="app-header__menu-section app-header__menu-section--danger">
                        <button type="button" onClick={() => void onLogout()}>
                          Выйти
                        </button>
                      </div>
                    </div>
                  </div>,
                  portalTarget,
                )
              : null}
          </>
        ) : (
          <a href={buildAuthUrl()} className="btn primary app-header__login">
            Войти
          </a>
        )}
      </div>
    </header>
  )
}
