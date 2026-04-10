import UiSpinner from './UiSpinner'

type Props = {
  dismissibleVisible: boolean
  onShowDismissible: () => void
  onHideDismissible: () => void
}

export default function UiKitStaticSections({ dismissibleVisible, onShowDismissible, onHideDismissible }: Props) {
  return (
    <>
      <section className="ui-section" aria-labelledby="section-buttons">
        <h2 id="section-buttons" className="ui-section-title">Кнопки</h2>
        <div className="ui-row">
          <button type="button" className="btn primary">Primary</button>
          <button type="button" className="btn btn-secondary-accent">Secondary</button>
          <button type="button" className="btn">Neutral</button>
          <button type="button" className="btn danger">Danger</button>
          <button type="button" className="btn btn-success">Success</button>
          <button type="button" className="btn btn-ghost">Ghost</button>
          <button type="button" className="btn primary" disabled>Disabled</button>
        </div>
        <div className="ui-divider" />
        <div className="ui-row">
          <button type="button" className="btn primary btn-sm">Маленькая</button>
          <button type="button" className="btn primary btn-lg">Большая</button>
        </div>
      </section>

      <section className="ui-section" aria-labelledby="section-badges">
        <h2 id="section-badges" className="ui-section-title">Бейджи</h2>
        <div className="ui-row">
          <span className="ui-badge ui-badge-accent">Primary</span>
          <span className="ui-badge ui-badge-secondary">Secondary</span>
          <span className="ui-badge ui-badge-success">Успех</span>
          <span className="ui-badge ui-badge-warning">Внимание</span>
          <span className="ui-badge ui-badge-neutral">Нейтральный</span>
        </div>
      </section>

      <section className="ui-section" aria-labelledby="section-chips">
        <h2 id="section-chips" className="ui-section-title">Chip</h2>
        <div className="ui-row">
          <span className="ui-chip"><span className="ui-chip-label">Тег без крестика</span></span>
          <span className="ui-chip ui-chip-primary">
            <span className="ui-chip-label">Primary</span>
            <button type="button" className="ui-chip-remove" aria-label="Удалить тег">×</button>
          </span>
          <span className="ui-chip ui-chip-secondary">
            <span className="ui-chip-label">Secondary</span>
            <button type="button" className="ui-chip-remove" aria-label="Удалить тег">×</button>
          </span>
        </div>
      </section>

      <section className="ui-section" aria-labelledby="section-accordion">
        <h2 id="section-accordion" className="ui-section-title">Accordion</h2>
        <div className="ui-accordion-group">
          <details className="ui-accordion" open>
            <summary>Открытый блок</summary>
            <div className="ui-accordion-panel">Контент первой секции. Нативный <code>&lt;details&gt;</code> дает доступность из коробки.</div>
          </details>
          <details className="ui-accordion">
            <summary>Второй пункт</summary>
            <div className="ui-accordion-panel">Текст во втором блоке аккордеона.</div>
          </details>
          <details className="ui-accordion">
            <summary>Третий пункт</summary>
            <div className="ui-accordion-panel">Можно вкладывать ссылки и кнопки по необходимости.</div>
          </details>
        </div>
      </section>

      <section className="ui-section" aria-labelledby="section-alerts-extended">
        <h2 id="section-alerts-extended" className="ui-section-title">Alert</h2>
        <div className="ui-stack" style={{ width: '100%', maxWidth: 480 }}>
          <div className="ui-alert ui-alert-info" role="status">
            <span className="ui-alert-icon" aria-hidden>ℹ</span>
            <div className="ui-alert-body">
              <span className="ui-alert-title">Заголовок</span>
              <span>Короткое описание под заголовком.</span>
            </div>
          </div>
          {dismissibleVisible ? (
            <div className="ui-alert ui-alert-warning ui-alert-dismissible" role="alert">
              <div className="ui-alert-main">
                <span className="ui-alert-icon" aria-hidden>⚠</span>
                <span>Закрываемый алерт, крестик справа.</span>
              </div>
              <button type="button" className="ui-alert-close" aria-label="Закрыть уведомление" onClick={onHideDismissible}>×</button>
            </div>
          ) : (
            <button type="button" className="btn btn-sm" onClick={onShowDismissible}>Показать алерт снова</button>
          )}
          <div className="ui-alert ui-alert-info" role="status"><span className="ui-alert-icon" aria-hidden>ℹ</span><span>Информация: ссылка действует ограниченное время.</span></div>
          <div className="ui-alert ui-alert-success" role="status"><span className="ui-alert-icon" aria-hidden>✓</span><span>Профиль успешно сохранен.</span></div>
          <div className="ui-alert ui-alert-warning" role="alert"><span className="ui-alert-icon" aria-hidden>⚠</span><span>Сессия скоро истечет, обновите страницу.</span></div>
          <div className="ui-alert ui-alert-error" role="alert"><span className="ui-alert-icon" aria-hidden>✕</span><span>Не удалось подключиться к серверу.</span></div>
        </div>
      </section>

      <section className="ui-section" aria-labelledby="section-navbar">
        <h2 id="section-navbar" className="ui-section-title">Navbar</h2>
        <nav className="ui-navbar" aria-label="Пример навигации">
          <div className="ui-navbar-brand">Project</div>
          <ul className="ui-navbar-links">
            <li><a href="#ui-kit">На сайт</a></li>
            <li><a href="#ui-kit">Магазин</a></li>
            <li><a href="#ui-kit">Таблица</a></li>
          </ul>
          <div className="ui-navbar-actions">
            <button type="button" className="btn btn-sm btn-secondary-accent">RU</button>
            <button type="button" className="btn btn-sm danger">Выход</button>
          </div>
        </nav>
      </section>

      <section className="ui-section" aria-labelledby="section-breadcrumb">
        <h2 id="section-breadcrumb" className="ui-section-title">Breadcrumb</h2>
        <nav aria-label="Хлебные крошки">
          <ol className="ui-breadcrumb">
            <li><a href="#ui-kit">Главная</a></li>
            <li className="ui-breadcrumb-sep" aria-hidden>/</li>
            <li><a href="#ui-kit">Кабинет</a></li>
            <li className="ui-breadcrumb-sep" aria-hidden>/</li>
            <li aria-current="page"><span>UI kit</span></li>
          </ol>
        </nav>
      </section>

      <section className="ui-section" aria-labelledby="section-spinner">
        <h2 id="section-spinner" className="ui-section-title">Spinner</h2>
        <div className="ui-row" style={{ alignItems: 'center' }}>
          <span className="ui-spinner-label"><UiSpinner size="sm" />S</span>
          <span className="ui-spinner-label"><UiSpinner size="md" />M</span>
          <span className="ui-spinner-label"><UiSpinner size="lg" />L</span>
        </div>
        <p className="ui-muted" style={{ fontSize: '0.8rem', marginTop: '0.75rem' }}>Четыре маркера по орбите, вращение всей группы.</p>
      </section>

      <section className="ui-section" aria-labelledby="section-cards">
        <h2 id="section-cards" className="ui-section-title">Карточки</h2>
        <div className="ui-kit-grid">
          <div className="card">
            <h3 className="card-title">Простая карточка</h3>
            <p className="card-text">Карточка #19191C, подсветка сверху, легкое стекло с blur(3px).</p>
          </div>
          <div className="card">
            <div className="ui-card-header">
              <div className="ui-row" style={{ flex: 1, minWidth: 0 }}>
                <div className="ui-avatar ui-avatar-sm">И</div>
                <div className="ui-stack" style={{ gap: 2 }}>
                  <span className="card-title" style={{ textAlign: 'left', margin: 0, fontSize: '1rem' }}>Игрок</span>
                  <span className="ui-muted" style={{ fontSize: '0.8rem' }}>В сети</span>
                </div>
              </div>
              <span className="ui-badge ui-badge-success">Pro</span>
            </div>
            <p className="card-text" style={{ textAlign: 'left' }}>Карточка с шапкой: аватар, заголовок и бейдж справа.</p>
            <div className="ui-card-footer">
              <button type="button" className="btn btn-sm">Отмена</button>
              <button type="button" className="btn primary btn-sm">Ок</button>
            </div>
          </div>
        </div>
      </section>

      <section className="ui-section" aria-labelledby="section-forms">
        <h2 id="section-forms" className="ui-section-title">Поля и переключатель</h2>
        <div className="ui-stack">
          <div className="ui-field">
            <label className="ui-label" htmlFor="ui-kit-email">Email</label>
            <input id="ui-kit-email" className="ui-input" type="email" placeholder="you@example.com" autoComplete="off" />
            <span className="ui-hint">Подсказка под полем</span>
          </div>
          <div className="ui-field ui-field-error">
            <label className="ui-label" htmlFor="ui-kit-error">С ошибкой</label>
            <input id="ui-kit-error" className="ui-input" type="text" defaultValue="некорректное значение" aria-invalid="true" />
            <span className="ui-hint ui-hint-error">Заполните это поле</span>
          </div>
          <label className="ui-switch">
            <input type="checkbox" defaultChecked />
            <span className="ui-switch-track" aria-hidden />
            Уведомления по почте
          </label>
        </div>
      </section>

      <section className="ui-section" aria-labelledby="section-tooltip">
        <h2 id="section-tooltip" className="ui-section-title">Tooltip</h2>
        <div className="ui-row">
          <span className="ui-tooltip ui-tooltip--top">
            <button type="button" className="btn btn-sm" aria-describedby="ui-kit-tooltip-1">Сверху</button>
            <span id="ui-kit-tooltip-1" role="tooltip" className="ui-tooltip-popup">Подсказка появляется при наведении и фокусе клавиатурой.</span>
          </span>
          <span className="ui-tooltip ui-tooltip--bottom">
            <button type="button" className="btn btn-sm" aria-describedby="ui-kit-tooltip-2">Снизу</button>
            <span id="ui-kit-tooltip-2" role="tooltip" className="ui-tooltip-popup">Вариант размещения снизу от триггера.</span>
          </span>
        </div>
      </section>

      <section className="ui-section" aria-labelledby="section-search">
        <h2 id="section-search" className="ui-section-title">Search</h2>
        <div className="ui-search" role="search">
          <span className="ui-search-icon" aria-hidden>⌕</span>
          <input className="ui-search-input" type="search" name="ui-kit-search" placeholder="Поиск по сайту…" autoComplete="off" aria-label="Поиск" />
        </div>
      </section>

      <section className="ui-section" aria-labelledby="section-radio">
        <h2 id="section-radio" className="ui-section-title">Radio</h2>
        <fieldset className="ui-radio-group">
          <legend className="ui-radio-legend">Выберите режим</legend>
          <label className="ui-radio"><input type="radio" name="ui-kit-radio-demo" value="survival" defaultChecked /><span className="ui-radio-mark" aria-hidden /> <span>Выживание</span></label>
          <label className="ui-radio"><input type="radio" name="ui-kit-radio-demo" value="creative" /><span className="ui-radio-mark" aria-hidden /> <span>Креатив</span></label>
          <label className="ui-radio"><input type="radio" name="ui-kit-radio-demo" value="spectator" /><span className="ui-radio-mark" aria-hidden /> <span>Наблюдатель</span></label>
        </fieldset>
      </section>

      <section className="ui-section" aria-labelledby="section-progress">
        <h2 id="section-progress" className="ui-section-title">Прогресс</h2>
        <div className="ui-stack" style={{ width: '100%', maxWidth: 360 }}>
          <progress value={62} max={100}>62%</progress>
          <p className="ui-muted" style={{ fontSize: '0.85rem' }}>Нативный <code style={{ fontSize: '0.8em' }}>&lt;progress&gt;</code> с акцентом.</p>
        </div>
      </section>
    </>
  )
}
