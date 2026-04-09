# Frontend Astro Architecture

Фронтенд переведён на Astro с React islands и Tailwind CSS. Цель структуры: лендинг, личный кабинет, админка и будущие страницы должны добавляться через file-based routing Astro, а React использоваться только там, где нужна клиентская интерактивность.

## Routing

Astro routes лежат в `frontend/src/pages`:

- `/` -> `src/pages/index.astro`
- `/auth` -> `src/pages/auth.astro`
- `/profile` -> `src/pages/profile.astro`
- `/profile/edit` -> `src/pages/profile/edit.astro`
- `/cabinet` -> `src/pages/cabinet/index.astro`
- `/ui-kit` -> `src/pages/ui-kit.astro`
- `/token` -> `src/pages/token.astro`

React-компоненты, которые живут рядом с Astro routes, имеют `_`-префикс: например `src/pages/_ProfilePage.tsx`. Это намеренно: Astro игнорирует такие файлы как route files. Для новых React-компонентов лучше использовать `src/components`, `src/features`, `src/widgets`; `_`-файлы в `src/pages` допустимы только для временного/переходного кода рядом с route.

## Layouts

Базовый shell:

- `src/layouts/BaseLayout.astro` — HTML skeleton, глобальные CSS imports, `.astro-app-root`, `.app-root`.
- `src/layouts/PublicLayout.astro` — публичные страницы и будущий лендинг.
- `src/layouts/CabinetLayout.astro` — личный кабинет и будущие пользовательские разделы.

Для админки следует добавить отдельный `src/layouts/AdminLayout.astro`, а не расширять `CabinetLayout` условностями.

## React Islands

React подключён через `@astrojs/react`. Компоненты с browser state/API подключаются в Astro через директивы:

```astro
<ProfilePage client:load />
```

Текущие islands:

- `AuthPage` / `AuthStartPage` / `AuthCallbackPage` — авторизация, PoW, Discord redirect.
- `ProfilePage` — профиль, skin viewer, upload, nickname update.
- `TokenHandler` — legacy обработчик token из URL.
- `UserEditPage` — legacy standalone edit form.
- `UiKitPage` — интерактивная витрина компонентов.

Правило: если страница статичная или почти статичная, делать её `.astro`, а не React component. React оставлять для stateful UI, canvas/3D, загрузки файлов, форм с rich behavior.

## Tailwind CSS

Tailwind подключён через `@tailwindcss/vite` в `astro.config.mjs` и импортируется в `src/styles/global.css`:

```css
@import "tailwindcss";
@import "../index.css";
@import "../App.css";
```

Tailwind можно использовать для layout-level задач и новых страниц. Текущий ui-kit всё ещё описан CSS-классами в `src/ui/ui.css`; не нужно механически переписывать его в utility classes без отдельного решения.

Практический подход:

- Astro layout/page structure: Tailwind utilities допустимы.
- Design-system primitives: сначала использовать `src/ui/ui.css`.
- Сложные повторяемые блоки: выносить в `.astro` или React-компонент, не копировать большие наборы utility classes по страницам.

## UI Kit

Текущий новый дизайн описан в `docs/frontend-ui-kit-design.md`.

Ключевые правила для будущей разработки:

- Новый ui-kit scoped через `.ui-kit-page`.
- Основной стиль: тёмный фон, VHS overlay, острые углы, orange/blue акценты.
- `AuthStart` использует тот же стиль через `AuthFlowStages`.
- Не смешивать старые purple/Discord-gradient элементы со страницами нового ui-kit.
- Если новый кабинет/админка должны быть в новом стиле, сначала выделить shared ui shell из `UiKitPage.css`, а не импортировать page CSS из feature-компонентов.

## Backend Proxy

Astro dev proxy настроен в `astro.config.mjs`:

```js
const apiTarget = process.env.ASTRO_API_TARGET || process.env.VITE_API_TARGET || 'http://127.0.0.1:3001'
```

Root script `npm run dev` всё ещё ожидает frontend на порту `5173`, поэтому `frontend/package.json` запускает:

```json
"dev": "astro dev --host 0.0.0.0 --port 5173"
```

## Build And Validation

Основные команды:

- `npm run lint --workspace frontend`
- `npm run build --workspace frontend`
- `npm run frontend:build`

В песочнице Windows сборка может падать на Vite cache cleanup с `EPERM`. Вне песочницы build проходит.

Остаточный build warning: chunks larger than 500 kB. Это ожидаемо на текущем переходном этапе из-за client islands и зависимостей вроде 3D skin viewer. Когда начнётся разработка нового кабинета/админки, нужно разнести тяжёлые widgets по lazy islands/dynamic imports.
