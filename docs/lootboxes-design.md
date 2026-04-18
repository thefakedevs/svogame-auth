# Lootboxes

## Summary

The lootbox subsystem is built as a dedicated domain on top of the existing ownership model.

- A lootbox itself is a regular `AssetDefinition`.
- A player owns lootboxes through normal inventory storage.
- Opening a lootbox is a server-side operation that consumes one lootbox asset and grants one configured reward.
- The server separately chooses the real reward and separately generates the visual roulette feed returned to the client.

This keeps reward issuance consistent with the current `inventory` model while giving the client a ready-to-render opening payload.

## Asset Rules

Lootboxes are ordinary assets with strict validation:

- `asset_kind = lootbox`
- `ownership_model = stackable`
- `is_currency = false`

That means a lootbox can be granted, removed, and stored exactly like any other non-currency stackable asset.

## Configuration Model

### LootboxDefinition

One `LootboxDefinition` points to one existing lootbox asset.

Fields:

- `id`
- `asset_definition_id`
- `is_active`
- `metadata`
- `created_at`
- `updated_at`

### LootboxDropDefinition

Each drop row is one exact outcome.

- No amount ranges
- No duration ranges
- No fuzzy reward blobs

If the same asset can drop in multiple quantities, that is modeled as multiple rows.

Examples:

- `coin_default x100`
- `coin_default x500`
- `repair_kit x2`
- `subscription_plus for 7 days`
- `exclusive_banner` with `coin_default x250` duplicate compensation

Fields:

- `id`
- `lootbox_definition_id`
- `reward_asset_definition_id`
- `stackable_amount`
- `expirable_duration_seconds`
- `duplicate_compensation_amount`
- `weight`
- `title_i18n`
- `is_active`
- `sort_order`
- `created_at`
- `updated_at`

Reward support in the current implementation:

- currency `stackable`
- non-currency `stackable`
- non-currency `expirable`
- non-currency `entitlement`

For entitlement rewards, `duplicate_compensation_amount` is required. If the selected entitlement is already owned by the user, the opening still records the entitlement as the selected reward, but grants `coin_default` in that configured amount instead.

## Weights

Each drop entry has a positive integer `weight`.

The real chance is:

`drop.weight / sum(active_drop_weights)`

Example:

- `repair_kit x2`, weight `50`
- `repair_kit x5`, weight `30`
- `subscription_plus 1h`, weight `20`

Total weight = `100`

Probabilities:

- `repair_kit x2` -> `50%`
- `repair_kit x5` -> `30%`
- `subscription_plus 1h` -> `20%`

The API returns both `weight` and `totalWeight`, so frontend or admin tooling can compute percentages without storing floats in the database.

## Opening Flow

`POST /api/user/me/lootboxes/{asset_key}/open`

The server performs one transaction:

1. Validate the lootbox asset key and opening request.
2. Confirm the user exists and the lootbox definition is active.
3. Load active drop definitions.
4. Select the real reward by weights.
5. Remove exactly one lootbox from inventory.
6. Grant the reward through the typed ownership services, or grant default currency compensation for duplicate entitlement rewards.
7. Generate a roulette feed separately from the reward selection.
8. Force the selected reward into `winnerIndex` inside the feed.
9. Persist a `LootboxOpenOperation`.
10. Return selected reward, final granted reward, compensation flag, and feed payload to the client.

This preserves a clean separation:

- `selectedReward` is what the weighted drop selected and what the roulette should show
- `reward` is what was actually granted to the player
- `wasCompensated` tells the client when `reward` is duplicate compensation instead of the selected entitlement
- `feed` is presentation data

## Roulette Feed

The client may request a feed length through `feedLength`.

Rules:

- default = `100`
- allowed range = `1..=200`

The server:

- chooses the real reward first
- generates the feed independently
- chooses `winnerIndex`
- replaces that feed slot with the real reward

So the client never decides what dropped and never derives the reward from feed contents.

## Reward Application

Rewards are issued through the same ownership services that the rest of the project already uses.

- currency reward -> `wallet::credit`
- stackable reward -> `inventory::add_stackable`
- expirable reward -> `inventory::prolong_expirable`
- entitlement reward -> `inventory::grant_entitlement`
- duplicate entitlement reward -> `wallet::credit` for `coin_default`

This means lootboxes automatically inherit existing behavior and invariants.

Important expirable rule:

- if the reward asset is already active, prolong starts from current `expires_at`
- if it is already expired, prolong starts from `now`

So lootbox rewards behave consistently with admin prolongation and subscription purchase semantics.

## History and Audit

### LootboxOpenOperation

Each opening stores:

- target user
- opened lootbox
- selected drop
- final reward payload
- actor information
- feed length
- winner index
- full generated feed JSON
- timestamp

This supports:

- user history
- admin investigation
- replay/debug of what the client was told to render

### Audit log

Audit actions:

- `admin.lootbox.created`
- `admin.lootbox.updated`
- `admin.lootbox.drop_created`
- `admin.lootbox.drop_updated`
- `admin.lootbox.drop_deleted`
- `user.lootbox.opened`
- `admin.lootbox.opened_for_user`

Service-token initiated opens keep `actorServiceName` in metadata and in lootbox open history.

## Endpoints

### Public

- `GET /api/lootboxes`
- `GET /api/lootboxes/{lootbox_id}`

### User

- `GET /api/user/me/lootboxes`
- `GET /api/user/me/lootboxes/open-history`
- `POST /api/user/me/lootboxes/{asset_key}/open`

### Admin

- `GET /api/admin/lootboxes`
- `POST /api/admin/lootboxes`
- `GET /api/admin/lootboxes/{lootbox_id}`
- `PATCH /api/admin/lootboxes/{lootbox_id}`
- `POST /api/admin/lootboxes/{lootbox_id}/drops`
- `PATCH /api/admin/lootboxes/{lootbox_id}/drops/{drop_id}`
- `DELETE /api/admin/lootboxes/{lootbox_id}/drops/{drop_id}`
- `GET /api/admin/users/{user_id}/lootboxes/open-history`
- `GET /api/admin/lootboxes/open-history`

### Privileged open on behalf of player

- `POST /api/admin/users/{user_id}/lootboxes/{asset_key}/open`

This endpoint accepts:

- human superuser bearer token
- service token bearer token

It exists for server-side integrations such as the Minecraft plugin.

## Frontend Notes

- Treat `selectedReward` as the authoritative visual drop result.
- Treat `reward` as the authoritative grant result.
- When `wasCompensated = true`, show that `selectedReward` dropped but `reward` was issued as duplicate compensation.
- Use `feed` only for animation/presentation.
- Do not try to reconstruct probabilities from feed composition.
- Show localized titles from `selectedReward.title`, `reward.title`, and `feed[*].title`; they are already resolved server-side.
- `winnerIndex` is the slot the animation should stop on.
- A lootbox can disappear from `/api/user/me/lootboxes` immediately after opening if its amount reaches zero.

## Current Scope

Implemented now:

- typed lootbox definition tables
- typed drop rows with exact amount or exact duration
- weighted reward selection
- separate roulette feed generation
- currency rewards
- entitlement rewards
- duplicate entitlement compensation in default currency
- user open endpoint
- privileged open-for-user endpoint
- open history
- OpenAPI coverage
- userflow tests

Intentionally deferred:

- batched open operations
- analytics endpoints
- purchase/store integration
