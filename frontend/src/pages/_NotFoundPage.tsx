import { paths } from '../routes/paths'
import './NotFoundPage.css'

export default function NotFoundPage() {
  return (
    <div className="page not-found-page">
      <section className="card not-found-card">
        <span className="not-found-code">Error 404</span>
        <h1 className="card-title not-found-title">Страница не найдена</h1>
        <p className="card-text not-found-text">
          Проверьте адрес или перейдите в один из основных разделов.
        </p>
        <div className="not-found-actions">
          <a href={paths.home} className="btn primary">На главную</a>
          <a href={paths.profile} className="btn">Профиль</a>
        </div>
      </section>
    </div>
  )
}
