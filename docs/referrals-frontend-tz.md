# ТЗ для фронта: реферальная регистрация

## Цель

Добавить поддержку реферальных кампаний в пользовательский auth-flow, админку и личную страницу контентмейкера.

Backend уже поддерживает:

- публичное превью активной реферальной кампании;
- передачу реферального кода при первой регистрации;
- выдачу welcome pack только при первом создании аккаунта;
- админское создание, редактирование, отзыв кампаний и просмотр статистики;
- обезличенную дневную статистику для контентмейкера.

## Основные правила

- Реферальная награда выдается только при первой регистрации пользователя.
- Если пользователь уже существует, `referralCode` игнорируется backend'ом.
- Если кампания отозвана, в draft, еще не началась или уже закончилась, код не применяется.
- Код нормализуется backend'ом в uppercase.
- Допустимый формат кода: `A-Z`, `0-9`, `_`, `-`, длина 3-32 символа.
- Дневная статистика сейчас считается в UTC.

## Пользовательский flow

### Вход по ссылке

Фронт должен поддержать ссылку вида:

```text
/auth?ref=CODE
```

или любой будущий route-алиас, который в итоге приводит пользователя на auth-страницу с сохраненным кодом.

При открытии auth-страницы:

1. Прочитать `ref` из URL.
2. Сохранить код в session/local storage auth-flow.
3. Запросить превью кампании.
4. Показать пользователю welcome pack, если код активен.
5. При финальном `/api/auth/register` отправить код как `referralCode`.

### Ручной ввод кода

Если пользователь пришел без `ref`, после legal screen нужно дать поле "Есть реферальный код?".

Если пользователь вводит код руками:

- вызвать preview endpoint;
- показать название кампании и награды;
- при успешном preview отправить код в `/api/auth/register`;
- ручной ввод должен иметь приоритет над кодом из URL.

## Public API

### Превью реферальной кампании

```http
GET /api/referrals/{code}
```

Успешный ответ:

```json
{
  "code": "CREATOR-ONE",
  "title": "Creator One Welcome",
  "contentCreatorUserId": null,
  "rewards": [
    {
      "assetKey": "coin_default",
      "assetDisplayName": "Coins",
      "assetKind": "currency",
      "ownershipModel": "stackable",
      "isCurrency": true,
      "amount": 2,
      "durationSeconds": null,
      "metadata": {}
    }
  ]
}
```

Ошибки:

- `404` - код не найден или кампания сейчас неактивна.

UI-поведение:

- для `404` не блокировать регистрацию, но убрать выбранный код или предложить ввести другой;
- если код был из URL, показать спокойное состояние "код недоступен";
- если код был введен руками, показать ошибку рядом с полем.

## Auth API

### Финальная регистрация

```http
POST /api/auth/register
```

Payload без рефералки остается валидным:

```json
{
  "registrationToken": "...",
  "acceptedUserAgreement": true,
  "acceptedPrivacyPolicy": true
}
```

Payload с рефералкой:

```json
{
  "registrationToken": "...",
  "acceptedUserAgreement": true,
  "acceptedPrivacyPolicy": true,
  "referralCode": "CREATOR-ONE",
  "referralSource": "link"
}
```

`referralSource`:

- `link` - код пришел из ссылки;
- `manual` - код введен пользователем руками.

Если welcome pack был применен, в успешном auth response будет поле `referral`:

```json
{
  "status": "authorized",
  "accessToken": "...",
  "id": "...",
  "username": "...",
  "avatarUrl": "...",
  "deliveryMethod": "redirect",
  "deliveryTarget": "/profile",
  "referral": {
    "code": "CREATOR-ONE",
    "title": "Creator One Welcome",
    "contentCreatorUserId": null,
    "rewards": []
  }
}
```

UI-поведение:

- если `referral` есть, показать экран/тост "Welcome pack получен";
- если `referral` нет, продолжить обычный flow;
- если `/api/auth/register` вернул `400` из-за кода, оставить пользователя на registration step и дать исправить/убрать код.

## Админка

Нужна отдельная секция реферальных кампаний.

### Список кампаний

```http
GET /api/admin/referral-campaigns?page=1&perPage=20
```

Ответ:

```json
{
  "items": [],
  "total": 0,
  "page": 1,
  "perPage": 20,
  "totalPages": 0
}
```

### Создание кампании

```http
POST /api/admin/referral-campaigns
```

Payload:

```json
{
  "code": "CREATOR-ONE",
  "title": "Creator One Welcome",
  "contentCreatorUserId": "uuid-or-null",
  "status": "active",
  "startsAt": null,
  "endsAt": null,
  "rewards": [
    {
      "assetKey": "coin_default",
      "amount": 2,
      "durationSeconds": null,
      "metadata": {}
    }
  ]
}
```

Поля:

- `contentCreatorUserId` optional;
- `status`: `active`, `draft`, `revoked`;
- `startsAt`/`endsAt` optional ISO datetime;
- `rewards` может быть пустым, если кампания нужна только для статистики.

Правила наград:

- currency asset: нужен `amount > 0`, `durationSeconds` не нужен;
- stackable non-currency: нужен `amount > 0`, `durationSeconds` не нужен;
- entitlement: `amount` и `durationSeconds` не передавать;
- expirable: нужен `durationSeconds > 0`, `amount` не нужен.

### Просмотр кампании

```http
GET /api/admin/referral-campaigns/{campaignId}
```

### Редактирование кампании

```http
PATCH /api/admin/referral-campaigns/{campaignId}
```

Payload частичный. Для полной замены наград передать `rewards`.

```json
{
  "title": "New title",
  "contentCreatorUserId": null,
  "status": "active",
  "startsAt": null,
  "endsAt": null,
  "rewards": []
}
```

### Отзыв кампании

```http
POST /api/admin/referral-campaigns/{campaignId}/revoke
```

После отзыва:

- public preview вернет `404`;
- новые регистрации по коду будут отклонены;
- старая статистика сохранится.

### Статистика кампании

```http
GET /api/admin/referral-campaigns/{campaignId}/stats?from=2026-06-01T00:00:00Z&to=2026-06-08T23:59:59Z
```

Ответ:

```json
{
  "total": 1,
  "buckets": [
    {
      "date": "2026-06-08",
      "registrations": 1
    }
  ]
}
```

График должен показывать дни с нулем, backend их возвращает.

## Кабинет контентмейкера

### Проверка наличия реферальных кампаний

Чтобы понять, показывать ли игроку кнопку/раздел рефералок, использовать:

```http
GET /api/referrals/me/campaigns
```

Требуется bearer token обычного пользователя.

Ответ, если у игрока нет привязанных кампаний:

```json
{
  "items": [],
  "total": 0
}
```

Ответ, если кампании есть:

```json
{
  "items": [
    {
      "id": "campaign-uuid",
      "code": "CREATOR-ONE",
      "title": "Creator One Welcome",
      "status": "active",
      "isActive": true,
      "startsAt": null,
      "endsAt": null,
      "rewards": [
        {
          "assetKey": "coin_default",
          "assetDisplayName": "Coins",
          "assetKind": "currency",
          "ownershipModel": "stackable",
          "isCurrency": true,
          "amount": 2,
          "durationSeconds": null,
          "metadata": {}
        }
      ]
    }
  ],
  "total": 1
}
```

UI-логика:

- если `total > 0`, показывать кнопку/вход в раздел рефералок;
- внутри раздела показывать список кампаний пользователя;
- для шаринга/призыва использовать только кампании с `isActive: true`;
- кампании с `isActive: false` можно показывать как неактивные или скрывать из списка для шаринга.

### Статистика

```http
GET /api/referrals/me/stats?from=2026-06-01T00:00:00Z&to=2026-06-08T23:59:59Z
```

Требования:

- показывать только агрегаты;
- не показывать user ids, Discord ids, emails и другие персональные данные;
- UI: total registrations + daily chart.

## Что не реализовано в backend на текущем этапе

- События воронки `visit`, `auth_started`, `launcher_download_clicked`.
- Список конкретных регистраций по кампании.
- Фильтрация админского списка кампаний по creator/code/status.
- Отдельная выдача или retry failed rewards.

Эти части не закладывать как обязательные для первого фронтового релиза.
