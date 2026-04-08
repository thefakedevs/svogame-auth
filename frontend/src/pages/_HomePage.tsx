import { paths } from '../routes/paths'
import './HomePage.css'

export default function HomePage() {
  return (
    <div className="page home-page">
      <section className="home-hero card">
        <span className="ui-badge ui-badge-accent">SVOCraft Account</span>
        <h1 className="card-title home-hero__title">Игровой аккаунт SVOCraft</h1>
        <p className="card-text home-hero__lead">
          Единый вход, профиль и управление игровым аккаунтом. Позже здесь появятся сквады, экономика и остальные разделы.
        </p>
        <div className="home-hero__actions">
          <a href={paths.auth} className="btn primary">
            Войти через Discord
          </a>
          <a href={paths.profile} className="btn">
            Открыть профиль
          </a>
        </div>
      </section>
      <section className="home-grid" aria-label="Разделы">
        <article className="card">
          <h2 className="card-title">Профиль</h2>
          <p className="card-text">Никнейм, скин и основные данные учетной записи.</p>
        </article>
        <article className="card">
          <h2 className="card-title">Настройки</h2>
          <p className="card-text">Управление никнеймом, скином и переход в раздел сквада.</p>
        </article>
      </section>
    </div>
  )
}
