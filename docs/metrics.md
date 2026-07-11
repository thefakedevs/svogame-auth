# Игровые метрики

## Архитектура

Pipeline встроен в существующий Rust backend без отдельного сервиса:

1. Axum принимает raw `application/x-ndjson` (фактический формат мода) или `multipart/form-data` (первое файловое поле, независимо от имени).
2. `TimelineStream` читает body чанками, одновременно считает SHA-256 и разбирает завершённые строки. В памяти остаются только незавершённая строка и state матча, но не весь файл и не список событий.
3. `TimelineAggregator` хранит текущие команды, presence-интервалы, damage context, уникальные vehicle entity и per-player counters.
4. После полной строгой валидации SeaORM одной транзакцией сохраняет ingestion, match, игроков, nickname history, participation, per-match stats и Discord outbox. До commit матч не виден read API/leaderboard.
5. Read API агрегирует только `completed` матчи. Для любого периода используется `metric_match.started_at` в UTC.
6. Фоновый worker забирает Discord outbox, отправляет embed через существующий bot client и сохраняет результат.

Сырые NDJSON не сохраняются: остаются точный SHA-256, byte/event counts, warnings и нормализованные агрегаты. Это уменьшает объём хранения; повторный перерасчёт требует повторной загрузки исходного файла. Другой файл для уже существующего game ID отклоняется, поэтому автоматическая замена опубликованной статистики не происходит.

## Реальный формат

Дискриминатор события — поле `type`. Поддерживаются:

- `game_started`, `game_ended`;
- `player_joined`, `player_left`, `player_team_changed`;
- `player_damage`, `player_death`;
- `vehicle_damage`, `vehicle_destroyed`;
- `money_movement` распознаётся и учитывается в event count, но не изменяет боевые метрики;
- неизвестный type сохраняет обработку и создаёт warning.

Неизвестные дополнительные JSON-поля разрешены. Некорректный JSON, отсутствующее обязательное поле, неверный UUID, отрицательное/не finite число, несовпадающий `gameId` или дублирующий `game_started`/`game_ended` отменяют ingestion целиком. Пустые строки разрешены.

События обрабатываются в исходном порядке. Одинаковый `timestampMs` использует номер строки как tie-breaker. Уменьшение timestamp создаёт warning, но исходный порядок сохраняется: в реальных файлах обнаружены только 14 сдвигов назад на 1 мс, поэтому сортировка всего файла в RAM не нужна.

## Схема БД

Миграция `m20260710_000001_create_metrics_tables` безопасно создаёт:

- `metric_ingestion`: уникальный `game_id`, SHA-256, status, byte/event counts, warnings;
- `metric_match`: карта, UTC start/end, исходные Unix milliseconds, duration, winner, processing status;
- `metric_player`: player UUID и последний nickname;
- `metric_player_nickname`: история nickname с unique `(player_id, nickname)`;
- `metric_match_player`: nickname в матче, initial/final team, team changes, joins/leaves и time in game;
- `metric_player_match_stat`: исходные counters/damage и JSON breakdowns по weapon/source/vehicle;
- `metric_discord_outbox`: ровно одна delivery на game ID, attempts, retry time, Discord IDs и последняя ошибка.

Внешние ключи имеют cascade для дочерних match/player данных. Уникальные ограничения защищают ingestion, participation, per-match stat и nickname history. Индексы покрывают start/status period filtering, `(player_id, game_id)` и due outbox lookup. Таблицы новые, поэтому миграция не блокирует перестроением существующие production-данные. `down` удаляет их в обратном FK-порядке.

## Конфигурация

| Переменная | Default | Назначение |
| --- | --- | --- |
| `METRICS_INGEST_SECRET` | отсутствует | Raw/Bearer secret. Без него endpoint закрыт с 503. |
| `METRICS_UPLOAD_MAX_BYTES` | `16777216` | Максимум байт самого NDJSON. |
| `METRICS_UPLOAD_TIMEOUT_SECONDS` | `60` | Общий timeout чтения upload. |
| `METRICS_DISCORD_CHANNEL_ID` | отсутствует | Канал итоговых embeds. |
| `METRICS_DISCORD_RETRY_INTERVAL_SECONDS` | `60` | Интервал повторной попытки. |
| `METRICS_DISCORD_MAX_ATTEMPTS` | `10` | После этого outbox получает `failed`. |
| `METRICS_PUBLIC_BASE_URL` | `https://svocraft.xyz` | Base URL ссылки на match API. |

Также Discord delivery использует существующие `DISCORD_BOT_TOKEN`, `DISCORD_PROXY` и `DISCORD_HTTP_TIMEOUT_MS`. `DISCORD_API_BASE_URL` по умолчанию равен официальному API v10 и переопределяется только для изолированных integration tests. Секрет ingest никогда не логируется и не возвращается. Сравнение raw/Bearer значения выполняется через HMAC-SHA256 verification. В проекте нет общего rate-limiter; защита здесь — secret, body limit и timeout. На reverse proxy отключена request buffering для streaming upload.

## Ingestion API

Production route:

```text
POST /api/metrics/timeline/{game_id}
```

Фактический мод отправляет raw body:

```bash
curl --location 'http://127.0.0.1:3001/api/metrics/timeline/6ad2866c-208b-4a90-945a-ac6055e5ea23' \
  --header 'Authorization: replace-with-secret' \
  --header 'Content-Type: application/x-ndjson' \
  --data-binary '@/path/to/timeline.ndjson'
```

Multipart также поддержан:

```bash
curl --location 'http://127.0.0.1:3001/api/metrics/timeline/6ad2866c-208b-4a90-945a-ac6055e5ea23' \
  --header 'Authorization: Bearer replace-with-secret' \
  --form 'file=@/path/to/timeline.ndjson;type=application/x-ndjson'
```

Успех нового файла возвращает 201 и `status=processed` либо `incomplete`. Точное повторение возвращает 200 и `status=duplicate`. Другой hash для того же `game_id` возвращает 409. Ошибка структуры — 422, отсутствие/неверный secret — 401, превышение размера — 413.

## Read API

- `GET /api/metrics/matches/{game_id}`
- `GET /api/metrics/matches/{game_id}/players`
- `GET /api/metrics/matches/{game_id}/players/{player_id}`
- `GET /api/metrics/players/{player_id}`
- `GET /api/metrics/players/{player_id}/matches`
- `GET /api/metrics/players/{player_id}/stats`
- `GET /api/metrics/leaderboard`


Player match/stats и leaderboard принимают RFC3339 `from`/`to`. Удобные rolling periods: `day` = последние 24 часа, `week` = 7 дней, `month` = 30 дней, `all`. Explicit range нельзя комбинировать с period, кроме `all`; обе границы включительны. Период всегда определяется по времени начала матча.

Leaderboard параметры: `metric`, `period`/`from`/`to`, `limit` (1..100, default 50), `offset` (>=0), `minMatches` (>=0), `sort=asc|desc`. Метрики: `kills`, `deaths`, `assists`, `vehicle_destructions`, `kd`, `kda`, `damage_dealt`, `damage_per_minute`, `win_rate`, `headshots`, `time_in_game`. Tie-breakers: выбранная metric, kills desc, deaths asc, player UUID asc. `rank` глобальный (`offset + index + 1`).

## Правила метрик

- Player identity — только `playerId`; nickname не объединяет игроков. В матче и глобально хранится nickname history.
- Spectators/пустая команда исключаются из team totals/win-loss, но player и его события не теряются.
- `time_in_game_ms` — сумма всех `[joined,left]`; повторные подключения поддержаны. Открытый интервал закрывается `game_ended`. В incomplete матче он не выдумывается.
- `damage_dealt` получает любой non-null attacker; `damage_taken` — victim. Self damage входит в dealt/taken и отдельно в `self_damage`, но не во friendly. Friendly определяется по командам в момент события.
- `headshots` — число headshot damage events; `headshot_damage` — их damage sum.
- Обычный kill: non-null, не self killer и разные известные команды (либо команда одной стороны неизвестна). Teamkill не входит в kills.
- `killerId == victimId` — suicide. `killerId == null` — одновременно suicide и `environment_death`; environment является явно названным подмножеством suicides, поэтому API не следует складывать их как независимые смерти.
- Assist выдаётся только при обычном kill: кандидат не killer/victim и его накопленный damage по victim составляет `>= 30%` всего damage context после предыдущей смерти. Context очищается после смерти и при team change victim.
- Vehicle damage/final hits дедуплицируются по `vehicleEntityId`; breakdown сохраняет type/weapon. Friendly vehicle damage определяется по attacker/owner teams.
- `vehicle_destroyed` дедуплицируется по `vehicleEntityId` на матч. `EMPTY`, null attacker и vehicle teamkill не увеличивают `vehicle_destructions`. Owner всё равно получает `vehicles_lost` для обычного non-EMPTY события.
- K/D = `kills / max(deaths, 1)`, KDA = `(kills + assists) / max(deaths, 1)`. DPM при нулевом времени равен 0. Win rate при нуле матчей равен 0. Производные коэффициенты округляются до 3 знаков, win rate до 2; damage и breakdowns API — до 3.
- Incomplete матч сохраняется для диагностики, но его stats, wins/losses и время открытых интервалов не попадают в period/lifetime leaderboard.

## Discord outbox

Outbox создаётся в той же DB-транзакции, что completed match. Worker атомарно переводит due row в `processing`, отправляет один embed (map, game ID, duration, winner, player count, totals и top-3 по kills/assists/vehicles), затем ставит `delivered` и сохраняет Discord channel/message IDs. Unicode режется по символам, title/description остаются ниже Discord limits.

Для повторной доставки используется стабильный nonce из game ID и `enforce_nonce=true`: Discord возвращает существующее сообщение при повторе nonce за последние минуты. Crash-stale `processing` повторно берётся после retry interval. Временные ошибки получают `retry`, после max attempts — `failed`. Отсутствие bot/channel не ломает приложение: completed outbox остаётся pending до появления конфигурации.

## Проверка и реальные данные

Команды:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace --all-targets --all-features
$env:SVO_METRICS_FIXTURES_DIR='C:\Users\baechka\Downloads\metrics'
cargo test -p auth --test metrics_real_data -- --ignored --nocapture
npm run frontend:build
```

Regression run 2026-07-10:

- 82/82 файла обработаны, 0 отклонено;
- 62 569 событий: 66 completed, 16 incomplete;
- 0 invalid JSON, 0 non-UTF-8, 0 mixed game ID;
- 14 timestamp regressions на 1 мс и 13 197 пар одинаковых timestamps;
- 18 player IDs использовали несколько nickname;
- 3 917 `vehicle_destroyed` событий соответствуют 1 763 уникальным `vehicleEntityId` (2 154 повтора).
- создано 2 186 warnings: 2 154 duplicate vehicle, 16 incomplete, 14 timestamp regressions и 2 подтверждённых расхождения `isTeamKill` с известными командами.

Сравнение с существующим Python-анализатором выполнено только по player IDs, которые он не отбрасывает:

| Метрика | Старый анализатор | Новый pipeline | Причина расхождения |
| --- | ---: | ---: | --- |
| deaths | 2 865 | 2 865 | Полное совпадение. |
| kills | 2 140 | 2 344 | Старый код теряет пустую/Spectators participation и не учитывает связанные смерти; новый сохраняет игрока и использует текущую team state. |
| assists | 477 | 568 | Та же потеря участников меняет допустимых кандидатов/context; threshold 30% сохранён. |
| vehicle destructions | 849 | 318 | Новый код дедуплицирует entity и исключает vehicle teamkills из обычных destructions. |

## Известные ограничения

- Raw NDJSON не архивируется, поэтому offline recalculation требует исходный файл; controlled replacement существующего game ID пока намеренно запрещён.
- Period leaderboard агрегирует индексированную выборку per-match rows в Rust. Это исключает N+1 и подходит текущему объёму, но при миллионах player-match rows следует добавить DB-side/materialized lifetime aggregates.
- Общего distributed rate limiter в проекте нет.
- `nonce` защищает от Discord-дубликатов в документированном окне «последних нескольких минут»; долгий retry после успешного Discord ответа и одновременной потери DB commit теоретически может создать повтор.
- Отдельная frontend-страница матча не добавлена; embed URL ведёт на JSON match endpoint.
