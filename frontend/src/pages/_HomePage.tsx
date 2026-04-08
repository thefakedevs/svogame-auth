import { paths } from '../routes/paths'
import './HomePage.css'

/**
 * Public landing placeholder. Replace or extend with real marketing content.
 */
export default function HomePage() {
  return (
    <div className="page home-page">
      <section className="home-hero card">
        <span className="ui-badge ui-badge-accent">SVO Account</span>
        <h1 className="card-title home-hero__title">Игровой аккаунт SVO</h1>
        <p className="card-text home-hero__lead">
          Единый вход, профиль, сквад и экономика. Лендинг будет развиваться здесь, вход и кабинет уже доступны.
        </p>
        <div className="home-hero__actions">
          <a href={paths.auth} className="btn primary">
            Войти через Discord
          </a>
          <a href={paths.cabinet} className="btn">
            Личный кабинет
          </a>
        </div>
      </section>
      <section className="home-grid" aria-label="Разделы">
        <article className="card">
          <h2 className="card-title">Профиль</h2>
          <p className="card-text">Никнейм, скин и данные аккаунта.</p>
        </article>
        <article className="card">
          <h2 className="card-title">Кабинет</h2>
          <p className="card-text">Будущие разделы сквадов, кошелька и владения.</p>
        </article>
      </section>
    </div>
  )
}
