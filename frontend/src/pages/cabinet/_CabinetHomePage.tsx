import { paths } from '../../routes/paths'
import './CabinetHomePage.css'

/**
 * Placeholder for the signed-in dashboard. Add widgets (squad, wallet, …) when APIs are wired.
 */
export default function CabinetHomePage() {
  return (
    <div className="page cabinet-home">
      <section className="card cabinet-home__hero">
        <span className="ui-badge ui-badge-secondary">Cabinet</span>
        <h1 className="card-title">Личный кабинет</h1>
        <p className="card-text cabinet-home__text">
          Основные настройки доступны в профиле. Разделы сквадов, кошелька и владения будут добавляться отдельными модулями.
        </p>
        <div className="cabinet-home__actions">
          <a href={paths.profile} className="btn primary">
            Открыть профиль
          </a>
          <a href={paths.home} className="btn">
            На главную
          </a>
        </div>
      </section>
      <section className="cabinet-home__grid" aria-label="Будущие разделы">
        <article className="card">
          <h2 className="card-title">Сквад</h2>
          <p className="card-text">Команды и приглашения.</p>
        </article>
        <article className="card">
          <h2 className="card-title">Экономика</h2>
          <p className="card-text">Кошелёк и транзакции.</p>
        </article>
      </section>
    </div>
  )
}
