export default function FooterSection() {
  const year = new Date().getFullYear()

  return (
    <footer className="landing-footer" aria-label="Подвал сайта">
      <nav className="landing-footer__nav" aria-label="Основные ссылки">
        <div className="landing-footer__group">
          <h2>Основное</h2>
          <ul>
            <li>
              <a href="/profile" data-cta="footer-profile">Профиль</a>
            </li>
            <li>
              <a href="/downloads" data-cta="footer-downloads">Скачать лаунчер</a>
            </li>
            <li>
              <a href="/wiki" data-cta="footer-wiki">Вики проекта</a>
            </li>
          </ul>
        </div>

        <div className="landing-footer__group">
          <h2>Документы</h2>
          <ul>
            <li>
              <a href="/legal/public_offer" data-cta="footer-privacy">Публичная оферта</a>
            </li>
            <li>
              <a href="/legal/user_agreement" data-cta="footer-terms">Пользовательское соглашение</a>
            </li>
            <li>
              <a href="/legal/privacy_policy" data-cta="footer-privacy">Политика конфиденциальности</a>
            </li>
            <li>
              <a href="/legal/refund_policy" data-cta="footer-privacy">Политика возвратов</a>
            </li>
          </ul>
        </div>

        <div className="landing-footer__group">
          <h2>Связь</h2>
          <ul>
            <li>
              <a href="/contacts" data-cta="footer-contacts">Контакты</a>
            </li>
            <li>
              <a href="https://discord.gg/UQQK4ykxMa" data-cta="footer-discord">Discord</a>
            </li>
            <li>
              <a href="https://t.me/sv0craft" data-cta="footer-telegram">Telegram</a>
            </li>
          </ul>
        </div>
      </nav>
      <div className="landing-footer__bottom">
        <span>© {year} SvoCraft Все права защищены</span>
        <span>Данный сервер не связан и не поддерживается Mojang AB. Все торговые марки и ресурсы принадлежат их законным владельцам.</span>
      </div>
    </footer>
  )
}
