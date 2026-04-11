import './ContactsPage.css'

export default function ContactsPage() {
  return (
    <main className="page contacts-page">
      <section className="contacts-hero card">
        <h1 className="card-title contacts-hero__title">Связь с SVOCraft</h1>
        <p className="card-text">
          Следите за новостями в социальных каналах и пишите в поддержку, если нужна помощь с аккаунтом, лаунчером или
          входом в игру.
        </p>
      </section>

      <section className="contacts-section" aria-labelledby="contacts-community-title">
        <h2 id="contacts-community-title" className="ui-section-title">Каналы проекта</h2>
        <div className="contacts-grid">
          <article className="card contacts-card">
            <div className="contacts-card__head">
              <h3 className="card-title">Discord-сервер</h3>
              <span className="ui-badge ui-badge-accent">Сообщество</span>
            </div>
            <p className="card-text">Сервер с основной коммуникацией игроков и самой актуальной информацией.</p>
            <div className="ui-card-footer">
              <a
                className="btn btn-sm btn-secondary-accent"
                href="https://discord.gg/UQQK4ykxMa"
                data-cta="contacts-discord"
                target="_blank"
                rel="noreferrer"
              >
                Открыть Discord
              </a>
            </div>
          </article>

          <article className="card contacts-card">
            <div className="contacts-card__head">
              <h3 className="card-title">Telegram-канал</h3>
              <span className="ui-badge ui-badge-accent">Новости</span>
            </div>
            <p className="card-text">Новости проекта, расписание ивентов, быстрые объявления по лаунчеру и серверам.</p>
            <div className="ui-card-footer">
              <a
                className="btn btn-sm btn-secondary-accent"
                href="https://t.me/sv0craft"
                data-cta="contacts-telegram-channel"
                target="_blank"
                rel="noreferrer"
              >
                Открыть Telegram
              </a>
            </div>
          </article>

          <article className="card contacts-card">
            <div className="contacts-card__head">
              <h3 className="card-title">TikTok</h3>
              <span className="ui-badge ui-badge-accent">Видео</span>
            </div>
            <p className="card-text">Короткие моменты матчей, хайлайты дуэлей и клипы с ивентов.</p>
            <div className="ui-card-footer">
              <a
                className="btn btn-sm btn-secondary-accent"
                href="https://www.tiktok.com/@sv0craft"
                data-cta="contacts-tiktok"
                target="_blank"
                rel="noreferrer"
              >
                Открыть TikTok
              </a>
            </div>
          </article>
        </div>
      </section>

      <section className="contacts-section" aria-labelledby="contacts-support-title">
        <h2 id="contacts-support-title" className="ui-section-title">Поддержка</h2>
        <div className="contacts-grid">
          <article className="card contacts-card">
            <div className="contacts-card__head">
              <h3 className="card-title">Почта поддержки</h3>
              <span className="ui-badge ui-badge-accent">Email</span>
            </div>
            <p className="card-text">Для вопросов по аккаунту, лаунчеру, оплатам и доступу к игре.</p>
            <div className="ui-card-footer">
              <a
                className="btn btn-sm btn-secondary-accent"
                href="mailto:svocraft@xyecoc.com"
                data-cta="contacts-support-email"
              >
                svocraft@xyecoc.com
              </a>
            </div>
          </article>

          <article className="card contacts-card">
            <div className="contacts-card__head">
              <h3 className="card-title">Поддержка в Telegram</h3>
              <span className="ui-badge ui-badge-accent">Support</span>
            </div>
            <p className="card-text">Быстрая помощь с любыми проблемами, входом и техническими ошибками.</p>
            <div className="ui-card-footer">
              <a
                className="btn btn-sm btn-secondary-accent"
                href="https://t.me/svocraft_sup"
                data-cta="contacts-support-telegram"
                target="_blank"
                rel="noreferrer"
              >
                Написать в Telegram
              </a>
            </div>
          </article>
        </div>
      </section>
    </main>
  )
}
