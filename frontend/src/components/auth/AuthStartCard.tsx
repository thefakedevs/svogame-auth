export default function AuthStartCard({
  onStart,
  errorMessage,
  isLoading,
}: {
  onStart: () => void;
  errorMessage?: string;
  isLoading?: boolean;
}) {
  return (
    <div className="page auth-start-page auth-pow-fullbleed">
      <div className="ui-kit-vhs" aria-hidden />
      <div className="ui-kit-page">
        <section className="card profile-state-card auth-start-card">
          <span className="ui-badge ui-badge-accent">Авторизация</span>
          <h1 className="card-title">Вход через Discord</h1>
          <p className="card-text">
            Войдите, чтобы открыть профиль и персональные разделы аккаунта.
          </p>
          {errorMessage ? <p className="card-text">{errorMessage}</p> : null}
          <div className="auth-start-card__actions mt-6 flex justify-end">
            <button className="btn primary" type="button" onClick={onStart} disabled={isLoading}>
              {isLoading ? "Подготовка..." : "Продолжить"}
            </button>
          </div>
        </section>
      </div>
    </div>
  );
}
