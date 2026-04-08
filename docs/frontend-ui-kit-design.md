# Frontend UI Kit Design Notes

Документ фиксирует текущее понимание нового дизайна фронтенда по `frontend/src/ui/ui.css`, `frontend/src/pages/UiKitPage.*` и экрану авторизации `AuthStartPage` через `AuthFlowStages`.

## Общая модель

Новый дизайн сейчас реализован как scoped UI system вокруг класса `.ui-kit-page`. Это важно: стили ui-kit не являются полным глобальным редизайном приложения и намеренно перекрывают старые глобальные `.btn`, `.card`, `progress` только внутри `.ui-kit-page`.

Экран `AuthStart` использует эту же систему через компонент `AuthFlowStages`: он импортирует `UiKitPage.css` и `ui.css`, затем рендерит контейнер с классами `.ui-kit-page.ui-auth-flow.ui-pow-check`. Поэтому для будущих экранов в этом стиле нужно либо явно входить в `.ui-kit-page`, либо выносить часть токенов/компонентов в более общий слой.

## Визуальный язык

База стиля: тёмный технологичный интерфейс с VHS-оверлеем, острыми углами, тонкими границами и акцентами orange/blue.

Основные токены:

- `--ui-primary: #f7a41d` — главный оранжевый акцент.
- `--ui-primary-hover: #dd9116`.
- `--ui-secondary: #223fff` — синий акцент для secondary/focus/progress.
- `--ui-secondary-hover: #213ad9`.
- `--ui-bg: #121212`.
- `--ui-card: #19191c`.
- `--ui-text: rgba(255, 255, 255, 0.92)`.
- `--ui-text-muted: rgba(255, 255, 255, 0.55)`.
- `--ui-border: rgba(255, 255, 255, 0.08)`.
- `--ui-glass-blur: blur(3px)`.

Ключевой принцип формы: острые углы. Внутри `.ui-kit-page` кнопки, карточки, инпуты, бейджи, модалки, прогресс и контролы используют `border-radius: 0`. Это отличается от старого фронтенда, где много скруглений и purple/Discord-gradient.

## Фон и слой VHS

Фон ui-kit задаётся в `UiKitPage.css` через `:root:has(.ui-kit-page)`: тёмный `#121212` с вертикальным оранжевым градиентом вниз. Сверху есть затемняющий `::before`, чтобы оранжевый низ не доминировал.

VHS-эффект задаётся отдельным элементом:

```tsx
<div className="ui-kit-vhs" aria-hidden />
```

Он фиксированный, `pointer-events: none`, с несколькими слоями: scanlines, вертикальная сетка, мерцание, лёгкий jitter и движущаяся полоса. Для `prefers-reduced-motion: reduce` анимации выключаются. На новых экранах в этом стиле VHS-слой должен быть один на экран, не внутри карточек.

## Компоненты ui-kit

Кнопки:

- Базовая `.btn`: прозрачная/тёмная, border `--ui-border`, uppercase, `letter-spacing: 0.04em`.
- `.btn.primary`: оранжевый фон, тёмный текст `#121212`.
- `.btn-secondary-accent`: прозрачная с синей границей.
- Danger/success имеют отдельные красный/зелёный акценты.
- Hover не двигает кнопку (`transform: none`), а меняет цвет/фон/границу.
- Focus внутри `.ui-kit-page` использует синий outline, для primary — оранжевый.

Карточки:

- `.card` внутри ui-kit — прямоугольная, без скругления.
- Фон: `#19191C` с лёгкой верхней подсветкой.
- Есть тонкая верхняя оранжевая световая линия через `::before`.
- Hover не должен сдвигать карточку, только слегка менять границу.

Формы:

- `.ui-field`, `.ui-label`, `.ui-input`, `.ui-hint`.
- Label маленький, uppercase, оранжевый.
- Input тёмный, прозрачный, с `blur(3px)`, без скругления.
- Focus у input — синяя граница.
- Error field — красная граница и красный hint.

Прогресс:

- Нативный `<progress>`.
- Высота базово 8px, без скругления.
- Value — синий градиент `--ui-secondary-hover -> --ui-secondary`.
- В auth/PoW варианте прогресс чуть толще: 10px.

Есть также готовые паттерны для badge, chip, alert, accordion, navbar, breadcrumb, spinner, tooltip, search, upload, radio, modal, switch, pagination. Их стоит переиспользовать по классам, а не копировать inline-стили.

## Типографика

В ui-kit базовая типографика остаётся от глобального `index.css`, но auth-flow задаёт свои переменные:

- `--ui-pow-font-sans: 'IBM Plex Sans', 'Inter', system-ui, sans-serif`.
- `--ui-pow-font-mono: 'IBM Plex Mono', ui-monospace, 'Cascadia Code', monospace`.

Для заголовков auth-flow используется `font-weight: 600`, размер `clamp(1.35rem, 3.4vw, 1.9rem)`, `letter-spacing: -0.03em`. Технические подписи PoW используют mono-шрифт, маленький размер и muted white.

Если продолжать этот стиль на новых экранах, лучше придерживаться умеренных размеров и не делать hero-тексты слишком крупными. Дизайн строится не на огромных landing-заголовках, а на компактных, центрированных интерфейсных состояниях.

## AuthStart / AuthFlowStages

`AuthStartPage` отвечает за логику:

1. Проверяет существующий token и редиректит в профиль, если auth-параметров нет.
2. Запрашивает `/api/auth/prepare` или читает polling payload.
3. Переводит state в `solving_pow`.
4. Запускает `solvePow`.
5. Сохраняет PoW data в auth store/sessionStorage.
6. Переводит state в `redirecting`.
7. Через 500ms делает `window.location.assign(oauthUrl)`.

`AuthFlowStages` отвечает за визуальный state machine:

- `loading`: spinner из 4 orbit-dot маркеров.
- `solving_pow`: lock icon, progress bar, сложность, скорость, ETA.
- `redirecting`: Discord icon и fallback-кнопка "Открыть Discord".
- `error`: error icon, красная подсветка, retry button.

Контейнер:

```tsx
<div className="ui-kit-page ui-auth-flow ui-pow-check vt-auth-root" data-phase={status}>
```

Стек:

```tsx
<div key={status} className="ui-pow-check-stack vt-auth-stack">
```

`key={status}` намеренно перемонтирует стек при смене стадии, чтобы запустить staged slot animation. Обновления прогресса PoW не меняют key, поэтому анимация не дёргается при каждом progress update.

Auth-flow должен оставаться одним экраном состояния, а не набором карточек. Центральная колонка ограничена примерно `min(32rem, 100%)`, весь экран центрируется с `min-height: min(78vh, 720px)`.

## Анимации

Основные анимации:

- VHS overlay: flicker, jitter, scanline movement, band movement.
- Spinner: вращение всей группы маркеров.
- Auth stage transition: `vt-auth-slot-in`, где дочерние элементы появляются снизу с задержками `0/55/110/165/220ms`.

Правило для будущих экранов: motion должен быть функциональным и неброским. Не добавлять отдельные декоративные blobs/orbs. Для reduced motion обязательно сохранять fallback.

## Практические правила для будущей работы

- Для новых экранов в этом стиле начинайте с контейнера `.ui-kit-page` и подключайте `../ui/ui.css`.
- Если нужен VHS-фон, добавляйте один `<div className="ui-kit-vhs" aria-hidden />` на уровень страницы.
- Не смешивайте старые глобальные purple/Discord-gradient карточки и кнопки с ui-kit-компонентами внутри одного экрана.
- Не добавляйте `border-radius`, если это не осознанное исключение: текущий стиль строится на острых прямоугольных формах.
- Основные акценты: orange для primary/brand/section, blue для focus/secondary/progress.
- Для статусов используйте существующие цвета: green success, orange warning, red error.
- Не делайте nested cards. Если нужен блок внутри карточки, используйте header/footer/divider/row/stack.
- Не создавайте новые SVG-иконки без необходимости. В auth-flow иконки уже встроены, для общих UI-состояний лучше сначала проверить существующие классы.
- Для progress/PoW используйте нативный `<progress>` и существующие классы `.ui-pow-check-*`.
- Для текстов писать пользовательский текст действия/состояния, не объяснять интерфейс.
- При переносе ui-kit на другие страницы учитывайте scope: без `.ui-kit-page` большинство новых правил не применится.

## Технические риски

- `:has(.ui-kit-page)` управляет фоном через root/body. Это удобно для демо и AuthStart, но при сложном роутинге может стать неожиданным глобальным side effect.
- `AuthFlowStages` импортирует CSS страницы `UiKitPage.css`. Это работает, но архитектурно лучше позже вынести shared background/VHS/layout стили в отдельный файл, например `ui/page-shell.css`.
- `ui.css` одновременно содержит базовые компоненты ui-kit и auth-specific классы `.ui-pow-check` / `.ui-auth-flow`. Если auth-flow будет развиваться отдельно, стоит разделить generic ui-kit и auth-flow styles.
- Текущий дизайн в auth-flow завязан на центрированный one-screen state. Для длинных форм и кабинета понадобится отдельный layout-паттерн в этом же стиле.
