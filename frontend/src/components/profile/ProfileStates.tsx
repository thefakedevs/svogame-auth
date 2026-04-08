import { currentAppPath, redirectToAuth } from '../../routes/auth'

export function ProfileUnauthorizedState() {
  return (
    <div className="ui-kit-page profile-page">
      <div className="profile-shell">
        <section className="card profile-state-card">
          <span className="ui-badge ui-badge-warning">Доступ</span>
          <h1 className="card-title">Нужно войти</h1>
          <p className="card-text">Профиль и личные настройки доступны только после входа через Discord.</p>
          <button className="btn primary" type="button" onClick={() => redirectToAuth(currentAppPath())}>
            Войти
          </button>
        </section>
      </div>
    </div>
  )
}

export function ProfileErrorState({ error, onRetry, onLogout }: {
  error: string
  onRetry: () => void
  onLogout: () => void
}) {
  return (
    <div className="ui-kit-page profile-page">
      <div className="profile-shell">
        <section className="card profile-state-card">
          <span className="ui-badge ui-badge-warning">Ошибка</span>
          <h1 className="card-title">Профиль недоступен</h1>
          <p className="card-text">{error || 'Не удалось загрузить данные аккаунта.'}</p>
          <div className="profile-actions">
            <button className="btn primary" type="button" onClick={onRetry}>
              Повторить
            </button>
            <button className="btn" type="button" onClick={onLogout}>
              Сбросить сессию
            </button>
          </div>
        </section>
      </div>
    </div>
  )
}
