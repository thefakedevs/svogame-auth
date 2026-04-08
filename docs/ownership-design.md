# Ownership Design

## Goal

Build a typed ownership subsystem for game assets with:

- a shared asset catalog;
- explicit separation between `AssetKind` and `OwnershipModel`;
- dedicated storage models for `Stackable`, `Entitlement`, and `Expirable`;
- a separate `wallet` subsystem for currencies;
- full audit trail for both inventory and wallet changes;
- first-class admin operations with actor attribution.

The design is intended for `Rust + axum + SeaORM + Postgres`.

## Bounded Contexts

### 1. Asset Catalog

Owns the canonical definition of a game asset.

- What asset is this?
- Can users see it?
- Can users purchase it?
- Is it a currency?
- Which ownership model is valid for it?

### 2. Inventory

Owns non-currency user possessions.

- `Stackable` assets with `amount`
- `Entitlement` assets with binary ownership
- `Expirable` assets with `expires_at`

### 3. Wallet

Owns currency balances and transaction journal.

- current balance per currency
- append-only transaction history
- reconciliation-friendly admin adjustments

### 4. Ownership Audit

Owns business-level history for changes in inventory and wallet.

This is separate from the existing global `audit_log`: the global log can still record high-level admin actions, but ownership needs domain-specific journals that are queryable by user, asset, operation type, actor, and time.

## Core Domain Model

### AssetDefinition

Catalog record describing a game asset.

Fields:

- `id: Uuid`
- `key: String`
  Stable code name, unique, machine-readable, for example `gold`, `premium_30d`, `skin_dragon_red`
- `display_name: String`
- `description: Option<String>`
- `asset_kind: AssetKind`
- `ownership_model: OwnershipModel`
- `is_currency: bool`
- `is_user_purchasable: bool`
- `is_public: bool`
- `is_active: bool`
- `metadata: Json`
- `created_at: DateTime<Utc>`
- `updated_at: DateTime<Utc>`

Constraints:

- `key` is globally unique
- `is_currency = true` implies `ownership_model = Stackable`
- `is_currency = true` implies the asset must live in `wallet`, not `inventory`
- `ownership_model` is immutable after first issuance in practice; if changed at all, it should be treated as a migration event, not a casual patch

### AssetKind

Semantic domain classification.

Examples:

- `Item`
- `Skin`
- `Subscription`
- `Cosmetic`
- `Lootbox`
- `Currency`
- `Ticket`
- `Token`

Recommendation for Rust:

- keep `AssetKind` as a closed enum for system-recognized kinds;
- if extensibility becomes critical later, add `Other(String)` only deliberately, not from day one.

### OwnershipModel

Ownership semantics.

Variants:

- `Stackable`
- `Entitlement`
- `Expirable`

### OwnershipActor

Who initiated the change.

```rust
enum OwnershipActorKind {
    System,
    User,
    Admin,
}

struct OwnershipActor {
    kind: OwnershipActorKind,
    user_id: Option<Uuid>,
    service_name: Option<String>,
}
```

Rules:

- `System` may carry `service_name`, job name, or subsystem key
- `User` must carry `user_id`
- `Admin` must carry `user_id`

### OperationReason

Do not reduce reason to a free-form string only.

Use:

- `operation_type: enum`
- `reason_code: Option<String>`
- `reason_text: Option<String>`
- `metadata: Json`

That gives both structured reporting and human-readable investigations.

## Storage Model

Do not store all ownership shapes in one nullable table.

Use separate tables:

- `asset_definition`
- `user_stackable_asset`
- `user_entitlement`
- `user_expirable_asset`
- `inventory_operation`
- `wallet_balance`
- `wallet_transaction`

## Postgres Tables

### `asset_definition`

Columns:

- `id uuid primary key`
- `key varchar not null unique`
- `display_name varchar not null`
- `description varchar null`
- `asset_kind varchar not null`
- `ownership_model varchar not null`
- `is_currency boolean not null default false`
- `is_user_purchasable boolean not null default false`
- `is_public boolean not null default false`
- `is_active boolean not null default true`
- `metadata jsonb not null default '{}'::jsonb`
- `created_at timestamptz not null default now()`
- `updated_at timestamptz not null default now()`

Checks:

- `ownership_model in ('stackable','entitlement','expirable')`
- if `is_currency = true`, then `ownership_model = 'stackable'`

Indexes:

- unique index on `key`
- index on `(asset_kind, is_active)`
- index on `(ownership_model, is_active)`
- partial index on `is_public where is_public = true`

### `user_stackable_asset`

Current state for non-currency stackables.

Columns:

- `user_id uuid not null`
- `asset_definition_id uuid not null`
- `amount bigint not null`
- `updated_at timestamptz not null default now()`
- primary key `(user_id, asset_definition_id)`

Checks:

- `amount >= 0`

Rules:

- only allowed for assets where `ownership_model = stackable` and `is_currency = false`
- zero amount may either be deleted physically or preserved as `0`; recommendation: delete row on zero to keep state minimal, while history lives in `inventory_operation`

### `user_entitlement`

Current state for binary ownership.

Columns:

- `user_id uuid not null`
- `asset_definition_id uuid not null`
- `granted_at timestamptz not null`
- `granted_by_actor jsonb not null`
- `updated_at timestamptz not null default now()`
- primary key `(user_id, asset_definition_id)`

Rules:

- only allowed for assets where `ownership_model = entitlement`
- presence of row means ownership
- revocation deletes the row and writes journal entry

### `user_expirable_asset`

Current state for temporary ownership.

Columns:

- `user_id uuid not null`
- `asset_definition_id uuid not null`
- `expires_at timestamptz not null`
- `granted_at timestamptz not null`
- `last_extended_at timestamptz null`
- `granted_by_actor jsonb not null`
- `updated_at timestamptz not null default now()`
- primary key `(user_id, asset_definition_id)`

Rules:

- only allowed for assets where `ownership_model = expirable`
- one row per user+asset; prolong updates `expires_at`
- `expires_at <= now()` means present but inactive
- optional cleanup job may delete long-expired rows only if product agrees; default recommendation is to keep them until archival policy is defined

### `inventory_operation`

Append-only journal for all non-currency ownership changes.

Columns:

- `id bigserial primary key`
- `user_id uuid not null`
- `asset_definition_id uuid not null`
- `ownership_model varchar not null`
- `operation_type varchar not null`
- `actor_kind varchar not null`
- `actor_user_id uuid null`
- `actor_service_name varchar null`
- `delta_amount bigint null`
- `new_amount bigint null`
- `previous_expires_at timestamptz null`
- `new_expires_at timestamptz null`
- `reason_code varchar null`
- `reason_text varchar null`
- `metadata jsonb not null default '{}'::jsonb`
- `created_at timestamptz not null default now()`

Examples of `operation_type`:

- `stackable_added`
- `stackable_removed`
- `stackable_set`
- `entitlement_granted`
- `entitlement_revoked`
- `expirable_prolonged`
- `expirable_expiration_set`
- `expirable_revoked`

Indexes:

- `(user_id, created_at desc)`
- `(asset_definition_id, created_at desc)`
- `(operation_type, created_at desc)`

### `wallet_balance`

Current balance per user and currency.

Columns:

- `user_id uuid not null`
- `currency_asset_definition_id uuid not null`
- `balance bigint not null`
- `updated_at timestamptz not null default now()`
- primary key `(user_id, currency_asset_definition_id)`

Checks:

- `balance >= 0`

Rules:

- only assets with `is_currency = true`
- current state only, all mutations must be backed by `wallet_transaction`

### `wallet_transaction`

Append-only transaction ledger for currencies.

Columns:

- `id bigserial primary key`
- `user_id uuid not null`
- `currency_asset_definition_id uuid not null`
- `operation_type varchar not null`
- `actor_kind varchar not null`
- `actor_user_id uuid null`
- `actor_service_name varchar null`
- `delta bigint not null`
- `balance_after bigint not null`
- `reason_code varchar null`
- `reason_text varchar null`
- `metadata jsonb not null default '{}'::jsonb`
- `created_at timestamptz not null default now()`

Examples of `operation_type`:

- `credit`
- `debit`
- `adjustment`
- `reservation_released`
- `refund`
- `purchase`
- `reward`

Indexes:

- `(user_id, currency_asset_definition_id, created_at desc)`
- `(user_id, created_at desc)`
- `(operation_type, created_at desc)`

## Rust Domain Types

Recommended separation:

```rust
struct AssetDefinition {
    id: Uuid,
    key: AssetKey,
    display_name: String,
    description: Option<String>,
    kind: AssetKind,
    ownership_model: OwnershipModel,
    is_currency: bool,
    is_user_purchasable: bool,
    is_public: bool,
    is_active: bool,
    metadata: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

struct StackableHolding {
    user_id: Uuid,
    asset_definition_id: Uuid,
    amount: u64,
    updated_at: DateTime<Utc>,
}

struct EntitlementHolding {
    user_id: Uuid,
    asset_definition_id: Uuid,
    granted_at: DateTime<Utc>,
    granted_by: OwnershipActor,
    updated_at: DateTime<Utc>,
}

struct ExpirableHolding {
    user_id: Uuid,
    asset_definition_id: Uuid,
    expires_at: DateTime<Utc>,
    granted_at: DateTime<Utc>,
    last_extended_at: Option<DateTime<Utc>>,
    granted_by: OwnershipActor,
    updated_at: DateTime<Utc>,
}

enum InventoryHolding {
    Stackable(StackableHolding),
    Entitlement(EntitlementHolding),
    Expirable(ExpirableHolding),
}
```

Do not expose raw SeaORM models as domain decisions. Keep:

- entity layer for persistence;
- repository layer per ownership shape;
- service layer for business operations and invariants;
- handler layer for HTTP.

## Repository Layout

Suggested modules:

- `domains/ownership/mod.rs`
- `domains/ownership/catalog.rs`
- `domains/ownership/inventory.rs`
- `domains/ownership/wallet.rs`
- `domains/ownership/handlers.rs`
- `services/ownership/catalog_service.rs`
- `services/ownership/inventory_service.rs`
- `services/ownership/wallet_service.rs`
- `services/ownership/types.rs`
- `entities/asset_definition.rs`
- `entities/user_stackable_asset.rs`
- `entities/user_entitlement.rs`
- `entities/user_expirable_asset.rs`
- `entities/inventory_operation.rs`
- `entities/wallet_balance.rs`
- `entities/wallet_transaction.rs`

If the codebase keeps domain and service layers flatter, at minimum still split repository methods by table and service methods by bounded context.

## Invariants

### Asset invariants

- each asset has exactly one `ownership_model`
- currencies are always `Stackable`
- currencies never use inventory tables
- non-currencies never use wallet tables

### Stackable invariants

- amount is never negative
- removal requires sufficient amount
- `set` is allowed only for admin/system operations

### Entitlement invariants

- at most one active entitlement row per user+asset
- granting the same entitlement twice is idempotent
- revocation of absent entitlement is either `204` idempotent success or explicit domain error; recommendation: idempotent success for admin UX

### Expirable invariants

- at most one row per user+asset
- active state is derived from `expires_at > now()`
- prolong never shortens expiration
- prolong of an expired holding restarts from `now`, not from historical `expires_at`
- `set_expiration` may shorten or extend and should be reserved for admin/system

### Wallet invariants

- balance is never negative
- each balance mutation writes exactly one `wallet_transaction`
- `balance_after` in transaction must match persisted balance inside the same DB transaction
- admin adjustment uses explicit operation type and reason, never masquerades as reward/purchase

## Main Business Operations

### Catalog operations

#### Create asset definition

Input:

- key
- display name
- asset kind
- ownership model
- flags
- metadata

Validation:

- unique key
- currency compatibility
- metadata shape if asset-specific validation exists

#### Update asset definition

Mutable:

- display name
- description
- flags
- metadata
- maybe `is_active`

Usually immutable:

- `key`
- `ownership_model`
- `is_currency`

### Inventory operations

#### Grant entitlement

Behavior:

- validate asset exists and is `Entitlement`
- insert row if absent
- if already present, return current state without duplication
- write `inventory_operation`

#### Revoke entitlement

Behavior:

- delete row if present
- write `inventory_operation`

#### Add stackable

Behavior:

- validate asset exists and is non-currency `Stackable`
- lock current row with `FOR UPDATE` in Postgres
- upsert amount `current + delta`
- write `inventory_operation`

#### Remove stackable

Behavior:

- validate sufficient amount
- subtract amount
- if result is zero, optionally delete row
- write `inventory_operation`

#### Set stackable

Behavior:

- admin/system only
- set exact amount
- if zero, optionally delete row
- write `inventory_operation` with `new_amount`

#### Prolong expirable

Behavior:

- validate asset is `Expirable`
- if row absent, create new expiration from provided base
- if row present and active, extend from current `expires_at`
- if row present but expired, extend from current time `now`, not from historical `expires_at`
- write `inventory_operation` with old/new expiration

Important business rule:

- prolonging an already expired `Expirable` is treated as issuing a new temporary period;
- formula: `new_expires_at = now + duration`;
- only active expirable holdings accumulate from the existing `expires_at`.

#### Set expiration

Behavior:

- admin/system only
- hard-set exact `expires_at`
- if new expiration is in the past, asset becomes inactive but still historically present
- write `inventory_operation`

#### Revoke expirable

Two acceptable models:

1. delete current row and write journal entry;
2. set expiration to `now`.

Recommendation:

- use delete for explicit revocation;
- use `set_expiration` when the product intentionally wants an inactive but still present record.

### Wallet operations

#### Credit currency

Behavior:

- validate asset is a currency
- lock `wallet_balance` row
- new balance = old balance + delta
- insert `wallet_transaction`
- commit atomically

#### Debit currency

Behavior:

- validate sufficient balance
- lock row
- subtract delta
- insert `wallet_transaction`
- commit atomically

#### Adjust balance

Behavior:

- admin/system only
- set exact balance by writing delta = `target - current`
- persist new balance
- write `wallet_transaction` with `operation_type = adjustment`

## API Design

All examples use `/api`.

### 1. Asset Catalog

#### `POST /api/admin/assets`

Create asset definition.

#### `GET /api/assets/{asset_id}`

Get public asset definition by id.

#### `GET /api/assets`

List asset definitions with filters:

- `q`
- `assetKind`
- `ownershipModel`
- `isCurrency`
- `isPublic`
- `isUserPurchasable`
- `isActive`
- `page`
- `perPage`

Public endpoint should return only `is_public = true` unless admin scope is used.

#### `GET /api/admin/assets/{asset_id}`

Admin-only full asset definition view.

#### `GET /api/admin/assets`

Admin-only list without public filtering.

#### `PATCH /api/admin/assets/{asset_id}`

Update mutable asset definition fields.

### 2. User Inventory

#### `GET /api/users/{user_id}/inventory`

Aggregated inventory response:

- `stackables`
- `entitlements`
- `expirables`

Likely admin-only for arbitrary `user_id`.

#### `GET /api/user/me/inventory`

Current user aggregated inventory.

#### `GET /api/users/{user_id}/inventory/contains/{asset_key}`

Returns typed ownership presence:

- for entitlement: owned or not
- for stackable: amount and `has_any`
- for expirable: active or not, `expires_at`

#### `GET /api/users/{user_id}/inventory/expirables/active`

Returns active expirable holdings.

#### `GET /api/users/{user_id}/inventory/entitlements`

Returns entitlements.

#### `GET /api/users/{user_id}/inventory/stackables`

Returns stackables.

For player self-access, mirror them under `/api/user/me/...`.

### 3. Admin Inventory Management

All endpoints below should require superuser.

#### `POST /api/admin/users/{user_id}/inventory/entitlements/{asset_key}/grant`

Body:

- `reasonCode`
- `reasonText`
- `metadata`

#### `POST /api/admin/users/{user_id}/inventory/entitlements/{asset_key}/revoke`

#### `POST /api/admin/users/{user_id}/inventory/stackables/{asset_key}/add`

Body:

- `amount`
- `reasonCode`
- `reasonText`
- `metadata`

#### `POST /api/admin/users/{user_id}/inventory/stackables/{asset_key}/remove`

#### `PUT /api/admin/users/{user_id}/inventory/stackables/{asset_key}`

Set exact amount.

#### `POST /api/admin/users/{user_id}/inventory/expirables/{asset_key}/prolong`

Body options:

- `durationSeconds`
- or exact `newExpiresAt` if the business wants precise control

Recommendation:

- support `duration_seconds` for prolong;
- keep exact timestamp for `set expiration`.

#### `PUT /api/admin/users/{user_id}/inventory/expirables/{asset_key}/expiration`

Set exact expiration.

#### `DELETE /api/admin/users/{user_id}/inventory/expirables/{asset_key}`

Revoke expirable.

#### `GET /api/admin/users/{user_id}/inventory/history`

Query params:

- `assetKey`
- `operationType`
- `from`
- `to`
- `page`
- `perPage`

### 4. Wallet

#### `GET /api/user/me/wallet`

List all balances for current user.

#### `GET /api/user/me/wallet/{currency_key}`

Get single currency balance.

#### `GET /api/user/me/wallet/{currency_key}/transactions`

Get transaction history for one currency.

Optional admin mirrors:

- `GET /api/admin/users/{user_id}/wallet`
- `GET /api/admin/users/{user_id}/wallet/{currency_key}`
- `GET /api/admin/users/{user_id}/wallet/{currency_key}/transactions`

### 5. Admin Wallet Management

#### `POST /api/admin/users/{user_id}/wallet/{currency_key}/credit`

#### `POST /api/admin/users/{user_id}/wallet/{currency_key}/debit`

#### `PUT /api/admin/users/{user_id}/wallet/{currency_key}/balance`

Set exact balance by adjustment.

#### `GET /api/admin/users/{user_id}/wallet/{currency_key}/transactions`

Admin currency history.

## Response Shapes

### Asset response

```json
{
  "id": "uuid",
  "key": "premium_30d",
  "displayName": "Premium 30 Days",
  "description": "Temporary premium subscription",
  "assetKind": "subscription",
  "ownershipModel": "expirable",
  "isCurrency": false,
  "isUserPurchasable": true,
  "isPublic": true,
  "isActive": true,
  "metadata": {},
  "createdAt": "2026-04-08T00:00:00Z",
  "updatedAt": "2026-04-08T00:00:00Z"
}
```

### Aggregated inventory response

```json
{
  "userId": "uuid",
  "stackables": [
    {
      "assetKey": "lootbox_key",
      "amount": 12
    }
  ],
  "entitlements": [
    {
      "assetKey": "skin_dragon_red",
      "grantedAt": "2026-04-08T00:00:00Z"
    }
  ],
  "expirables": [
    {
      "assetKey": "premium_30d",
      "expiresAt": "2026-05-08T00:00:00Z",
      "isActive": true
    }
  ]
}
```

### Wallet balance response

```json
{
  "userId": "uuid",
  "currency": "gold",
  "balance": 1450,
  "updatedAt": "2026-04-08T00:00:00Z"
}
```

## Service Interfaces

Suggested Rust service boundaries:

```rust
trait AssetCatalogService {
    async fn create_definition(&self, cmd: CreateAssetDefinition) -> Result<AssetDefinition>;
    async fn get_definition(&self, id: Uuid) -> Result<AssetDefinition>;
    async fn list_definitions(&self, query: AssetDefinitionQuery) -> Result<Page<AssetDefinition>>;
    async fn update_definition(&self, id: Uuid, cmd: UpdateAssetDefinition) -> Result<AssetDefinition>;
}

trait InventoryService {
    async fn grant_entitlement(&self, cmd: GrantEntitlement) -> Result<EntitlementHolding>;
    async fn revoke_entitlement(&self, cmd: RevokeEntitlement) -> Result<()>;
    async fn add_stackable(&self, cmd: AddStackable) -> Result<StackableHolding>;
    async fn remove_stackable(&self, cmd: RemoveStackable) -> Result<StackableHolding>;
    async fn set_stackable(&self, cmd: SetStackable) -> Result<StackableHolding>;
    async fn prolong_expirable(&self, cmd: ProlongExpirable) -> Result<ExpirableHolding>;
    async fn set_expiration(&self, cmd: SetExpiration) -> Result<ExpirableHolding>;
    async fn revoke_expirable(&self, cmd: RevokeExpirable) -> Result<()>;
}

trait WalletService {
    async fn credit(&self, cmd: CreditCurrency) -> Result<WalletBalance>;
    async fn debit(&self, cmd: DebitCurrency) -> Result<WalletBalance>;
    async fn adjust_balance(&self, cmd: AdjustWalletBalance) -> Result<WalletBalance>;
}
```

Command objects should carry:

- target user
- asset or currency reference
- actor
- reason
- metadata
- idempotency token if later needed for external systems

## Transaction Boundaries

Each mutating operation should run in a single DB transaction:

1. resolve and validate `AssetDefinition`
2. lock current holding or balance row if needed
3. mutate current state
4. append operation journal row
5. optionally append high-level global audit row
6. commit

For Postgres, prefer:

- `SELECT ... FOR UPDATE` on balance and holding rows;
- database-side constraints for `amount >= 0` and `balance >= 0`;
- explicit conflict handling for first acquisition.

## Authorization Model

### User permissions

- user can read own inventory
- user can read own wallet
- user cannot mutate ownership directly through admin endpoints

### Admin permissions

- superuser can read and mutate any user inventory
- superuser can read and mutate any wallet
- all admin mutations require actor attribution

### System permissions

- background jobs, reward processors, shop flows, or event consumers can mutate via service layer with `OwnershipActorKind::System`

## Audit Trail Strategy

Use two layers:

### Domain journals

Authoritative source for ownership history:

- `inventory_operation`
- `wallet_transaction`

### Global admin audit log

Optional but useful for admin console traceability:

- `admin.inventory.entitlement_granted`
- `admin.inventory.entitlement_revoked`
- `admin.inventory.stackable_added`
- `admin.inventory.stackable_removed`
- `admin.inventory.stackable_set`
- `admin.inventory.expirable_prolonged`
- `admin.inventory.expirable_expiration_set`
- `admin.inventory.expirable_revoked`
- `admin.wallet.credited`
- `admin.wallet.debited`
- `admin.wallet.adjusted`

The global audit log should reference enough metadata to connect the action to the domain journal entry.

## Suggested Error Model

Domain errors:

- `AssetNotFound`
- `AssetInactive`
- `OwnershipModelMismatch`
- `CurrencyInventoryMismatch`
- `InsufficientStackableAmount`
- `EntitlementAlreadyOwned`
- `EntitlementNotOwned`
- `ExpirableNotOwned`
- `WalletInsufficientFunds`
- `InvalidExpiration`
- `UnsupportedAssetMutation`

HTTP mapping:

- `400` for malformed input or invalid operation for asset type
- `404` for missing user or asset
- `409` for state conflicts when idempotent success is not chosen
- `422` for domain validation failures like insufficient funds

## Implementation Notes For This Codebase

Recommended rollout order:

1. add new SeaORM entities for catalog, inventory, wallet journals and balances
2. add migrations for each table and key indexes
3. add `domains/ownership` router with read-only catalog and self-inventory endpoints first
4. add service layer for inventory and wallet mutations
5. add admin handlers and wire to existing superuser guard
6. add utoipa schemas for all request and response types
7. add integration tests around:
   - stackable add/remove/set
   - entitlement grant/revoke idempotency
   - expirable prolong and set expiration
   - wallet credit/debit/adjustment
   - audit rows being written atomically with state changes

## Final Recommendation

Treat ownership as one domain with two internal subdomains:

- `inventory` for non-currency assets
- `wallet` for currencies

Keep the asset catalog shared, but make current state and journals strongly typed and physically separate. That gives:

- clearer invariants;
- easier Rust modeling with enums and dedicated structs;
- cleaner SQL constraints;
- safer admin tooling;
- better auditability;
- less long-term pain than a universal nullable ownership table.
