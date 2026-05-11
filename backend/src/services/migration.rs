use sea_orm::prelude::Expr;
use sea_orm::sea_query::{ColumnDef, Table};
use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr, DeriveIden, Statement};
use sea_orm_migration::{MigrationName, MigrationTrait, SchemaManager};

pub struct CreateAuthRayTable;

impl MigrationName for CreateAuthRayTable {
    fn name(&self) -> &str {
        "m20251206_000001_create_auth_ray_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateAuthRayTable {
    async fn up(&self, manager: &SchemaManager) -> std::result::Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AuthRay::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AuthRay::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AuthRay::PowPrefix).string().not_null())
                    .col(
                        ColumnDef::new(AuthRay::PowComplexity)
                            .tiny_unsigned()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AuthRay::DeliveryMethod).string().not_null())
                    .col(ColumnDef::new(AuthRay::DeliveryTarget).string().not_null())
                    .col(
                        ColumnDef::new(AuthRay::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> std::result::Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AuthRay::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
#[allow(dead_code)]
enum AuthRay {
    Table,
    Id,
    PowPrefix,
    PowComplexity,
    DeliveryMethod,
    DeliveryTarget,
    RegistrationToken,
    PendingDiscordId,
    PendingUsername,
    PendingAvatarUrl,
    PendingEmail,
    CreatedAt,
}

pub struct AddAuthRayRegistrationColumns;

impl MigrationName for AddAuthRayRegistrationColumns {
    fn name(&self) -> &str {
        "m20260412_000023_add_auth_ray_registration_columns"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for AddAuthRayRegistrationColumns {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let statements = match backend {
            DatabaseBackend::Postgres | DatabaseBackend::Sqlite => vec![
                r#"ALTER TABLE "auth_ray" ADD COLUMN "registration_token" varchar NULL"#,
                r#"ALTER TABLE "auth_ray" ADD COLUMN "pending_discord_id" varchar NULL"#,
                r#"ALTER TABLE "auth_ray" ADD COLUMN "pending_username" varchar NULL"#,
                r#"ALTER TABLE "auth_ray" ADD COLUMN "pending_avatar_url" varchar NULL"#,
                r#"ALTER TABLE "auth_ray" ADD COLUMN "pending_email" varchar NULL"#,
            ],
            _ => return Ok(()),
        };

        for sql in statements {
            match manager
                .get_connection()
                .execute(Statement::from_string(backend, sql.to_string()))
                .await
            {
                Ok(_) => {}
                Err(error) if is_duplicate_column_error(&error) => {}
                Err(error) => return Err(error),
            }
        }

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

pub struct CreateUserTable;

impl MigrationName for CreateUserTable {
    fn name(&self) -> &str {
        "m20251206_000002_create_user_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateUserTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(User::Id).uuid().not_null().primary_key())
                    .col(
                        ColumnDef::new(User::DiscordId)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(User::Username).string().not_null())
                    .col(ColumnDef::new(User::AvatarUrl).string().null())
                    .col(ColumnDef::new(User::Email).string().null())
                    .col(
                        ColumnDef::new(User::AuthEpoch)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(User::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(User::IsSuperuser)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(User::SquadId).uuid().null())
                    .col(ColumnDef::new(User::DeactivationReason).string().null())
                    .col(
                        ColumnDef::new(User::LastLoginAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(User::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
    DiscordId,
    Username,
    AvatarUrl,
    Email,
    AuthEpoch,
    IsActive,
    IsSuperuser,
    SquadId,
    DeactivationReason,
    LastLoginAt,
    CreatedAt,
}

pub struct CreateLittlemiceCheckTable;

impl MigrationName for CreateLittlemiceCheckTable {
    fn name(&self) -> &str {
        "m20260510_000001_create_littlemice_check_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateLittlemiceCheckTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LittlemiceCheck::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LittlemiceCheck::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(LittlemiceCheck::PlayerUuid).uuid().not_null())
                    .col(
                        ColumnDef::new(LittlemiceCheck::ServiceTokenId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LittlemiceCheck::ServiceSystemName)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(LittlemiceCheck::Status).string().not_null())
                    .col(ColumnDef::new(LittlemiceCheck::FailureReason).string().null())
                    .col(ColumnDef::new(LittlemiceCheck::PushTokenHash).string().null())
                    .col(
                        ColumnDef::new(LittlemiceCheck::PushTokenExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LittlemiceCheck::RequestedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LittlemiceCheck::ReceivedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LittlemiceCheck::CompletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(ColumnDef::new(LittlemiceCheck::ScreenshotS3Key).string().null())
                    .col(
                        ColumnDef::new(LittlemiceCheck::ScreenshotContentType)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LittlemiceCheck::ScreenshotSizeBytes)
                            .big_integer()
                            .null(),
                    )
                    .col(ColumnDef::new(LittlemiceCheck::Screenshot2S3Key).string().null())
                    .col(
                        ColumnDef::new(LittlemiceCheck::Screenshot2ContentType)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LittlemiceCheck::Screenshot2SizeBytes)
                            .big_integer()
                            .null(),
                    )
                    .col(ColumnDef::new(LittlemiceCheck::LogS3Key).string().null())
                    .col(ColumnDef::new(LittlemiceCheck::LogContentType).string().null())
                    .col(
                        ColumnDef::new(LittlemiceCheck::LogSizeBytes)
                            .big_integer()
                            .null(),
                    )
                    .col(ColumnDef::new(LittlemiceCheck::ClientInfoText).text().null())
                    .col(
                        ColumnDef::new(LittlemiceCheck::ClientInfoSizeBytes)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LittlemiceCheck::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(LittlemiceCheck::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(LittlemiceCheck::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum LittlemiceCheck {
    Table,
    Id,
    PlayerUuid,
    ServiceTokenId,
    ServiceSystemName,
    Status,
    FailureReason,
    PushTokenHash,
    PushTokenExpiresAt,
    RequestedAt,
    ReceivedAt,
    CompletedAt,
    ScreenshotS3Key,
    ScreenshotContentType,
    ScreenshotSizeBytes,
    Screenshot2S3Key,
    Screenshot2ContentType,
    Screenshot2SizeBytes,
    LogS3Key,
    LogContentType,
    LogSizeBytes,
    ClientInfoText,
    ClientInfoSizeBytes,
    CreatedAt,
    UpdatedAt,
}

pub struct AddLittlemiceCheckScreenshot2Columns;

impl MigrationName for AddLittlemiceCheckScreenshot2Columns {
    fn name(&self) -> &str {
        "m20260510_000002_add_littlemice_check_screenshot2_columns"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for AddLittlemiceCheckScreenshot2Columns {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let statements = match backend {
            DatabaseBackend::Postgres | DatabaseBackend::Sqlite => vec![
                r#"ALTER TABLE "littlemice_check" ADD COLUMN "screenshot2_s3_key" varchar NULL"#,
                r#"ALTER TABLE "littlemice_check" ADD COLUMN "screenshot2_content_type" varchar NULL"#,
                r#"ALTER TABLE "littlemice_check" ADD COLUMN "screenshot2_size_bytes" bigint NULL"#,
            ],
            _ => return Ok(()),
        };

        for sql in statements {
            match manager
                .get_connection()
                .execute(Statement::from_string(backend, sql.to_string()))
                .await
            {
                Ok(_) => {}
                Err(error) if is_duplicate_column_error(&error) => {}
                Err(error) => return Err(error),
            }
        }

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

pub struct AddUserSuperuserColumn;

impl MigrationName for AddUserSuperuserColumn {
    fn name(&self) -> &str {
        "m20260407_000003_add_user_is_superuser_column"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for AddUserSuperuserColumn {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = match manager.get_database_backend() {
            DatabaseBackend::Postgres => {
                r#"ALTER TABLE "user" ADD COLUMN "is_superuser" boolean NOT NULL DEFAULT false"#
            }
            DatabaseBackend::Sqlite => {
                r#"ALTER TABLE "user" ADD COLUMN "is_superuser" boolean NOT NULL DEFAULT false"#
            }
            _ => return Ok(()),
        };

        match manager
            .get_connection()
            .execute(Statement::from_string(
                manager.get_database_backend(),
                sql.to_string(),
            ))
            .await
        {
            Ok(_) => Ok(()),
            Err(error) if is_duplicate_column_error(&error) => Ok(()),
            Err(error) => Err(error),
        }
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

fn is_duplicate_column_error(error: &DbErr) -> bool {
    let error_text = error.to_string().to_lowercase();
    error_text.contains("duplicate column")
        || error_text.contains("duplicate column name")
        || error_text.contains("already exists")
}

pub struct AddUserDeactivationReasonColumn;

impl MigrationName for AddUserDeactivationReasonColumn {
    fn name(&self) -> &str {
        "m20260407_000004_add_user_deactivation_reason_column"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for AddUserDeactivationReasonColumn {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = match manager.get_database_backend() {
            DatabaseBackend::Postgres => {
                r#"ALTER TABLE "user" ADD COLUMN "deactivation_reason" varchar NULL"#
            }
            DatabaseBackend::Sqlite => {
                r#"ALTER TABLE "user" ADD COLUMN "deactivation_reason" varchar NULL"#
            }
            _ => return Ok(()),
        };

        match manager
            .get_connection()
            .execute(Statement::from_string(
                manager.get_database_backend(),
                sql.to_string(),
            ))
            .await
        {
            Ok(_) => Ok(()),
            Err(error) if is_duplicate_column_error(&error) => Ok(()),
            Err(error) => Err(error),
        }
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

pub struct AddUserSquadIdColumn;

impl MigrationName for AddUserSquadIdColumn {
    fn name(&self) -> &str {
        "m20260407_000006_add_user_squad_id_column"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for AddUserSquadIdColumn {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = match manager.get_database_backend() {
            DatabaseBackend::Postgres => r#"ALTER TABLE "user" ADD COLUMN "squad_id" uuid NULL"#,
            DatabaseBackend::Sqlite => r#"ALTER TABLE "user" ADD COLUMN "squad_id" uuid NULL"#,
            _ => return Ok(()),
        };

        match manager
            .get_connection()
            .execute(Statement::from_string(
                manager.get_database_backend(),
                sql.to_string(),
            ))
            .await
        {
            Ok(_) => Ok(()),
            Err(error) if is_duplicate_column_error(&error) => Ok(()),
            Err(error) => Err(error),
        }
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

pub struct CreateAuditLogTable;

impl MigrationName for CreateAuditLogTable {
    fn name(&self) -> &str {
        "m20260407_000005_create_audit_log_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateAuditLogTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AuditLog::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AuditLog::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AuditLog::Action).string().not_null())
                    .col(ColumnDef::new(AuditLog::ActorUserId).uuid().null())
                    .col(ColumnDef::new(AuditLog::TargetUserId).uuid().null())
                    .col(ColumnDef::new(AuditLog::Reason).string().null())
                    .col(ColumnDef::new(AuditLog::Metadata).text().null())
                    .col(
                        ColumnDef::new(AuditLog::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AuditLog::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum AuditLog {
    Table,
    Id,
    Action,
    ActorUserId,
    TargetUserId,
    Reason,
    Metadata,
    CreatedAt,
}

pub struct CreateSquadTable;

impl MigrationName for CreateSquadTable {
    fn name(&self) -> &str {
        "m20260407_000007_create_squad_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateSquadTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Squad::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Squad::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Squad::LeaderUserId).uuid().not_null())
                    .col(ColumnDef::new(Squad::Name).string().not_null())
                    .col(ColumnDef::new(Squad::ImageKey).string().null())
                    .col(ColumnDef::new(Squad::ImageContentType).string().null())
                    .col(
                        ColumnDef::new(Squad::IsRestricted)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Squad::RestrictionReason).string().null())
                    .col(
                        ColumnDef::new(Squad::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Squad::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Squad::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Squad {
    Table,
    Id,
    LeaderUserId,
    Name,
    ImageKey,
    ImageContentType,
    IsRestricted,
    RestrictionReason,
    CreatedAt,
    UpdatedAt,
}

pub struct CreateSquadInviteTable;

impl MigrationName for CreateSquadInviteTable {
    fn name(&self) -> &str {
        "m20260407_000008_create_squad_invite_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateSquadInviteTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SquadInvite::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SquadInvite::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(SquadInvite::SquadId).uuid().not_null())
                    .col(ColumnDef::new(SquadInvite::InviterUserId).uuid().not_null())
                    .col(ColumnDef::new(SquadInvite::InvitedUserId).uuid().not_null())
                    .col(
                        ColumnDef::new(SquadInvite::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SquadInvite::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SquadInvite::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum SquadInvite {
    Table,
    Id,
    SquadId,
    InviterUserId,
    InvitedUserId,
    ExpiresAt,
    CreatedAt,
}

pub struct CreateUserRestrictionTable;

impl MigrationName for CreateUserRestrictionTable {
    fn name(&self) -> &str {
        "m20260407_000009_create_user_restriction_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateUserRestrictionTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserRestriction::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserRestriction::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(UserRestriction::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(UserRestriction::RestrictionKey)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(UserRestriction::Reason).string().null())
                    .col(
                        ColumnDef::new(UserRestriction::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserRestriction::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum UserRestriction {
    Table,
    Id,
    UserId,
    RestrictionKey,
    Reason,
    CreatedAt,
}

pub struct CreateAssetDefinitionTable;

impl MigrationName for CreateAssetDefinitionTable {
    fn name(&self) -> &str {
        "m20260408_000010_create_asset_definition_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateAssetDefinitionTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AssetDefinition::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AssetDefinition::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AssetDefinition::Key)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(AssetDefinition::DisplayName)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AssetDefinition::Description).string().null())
                    .col(
                        ColumnDef::new(AssetDefinition::AssetKind)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AssetDefinition::OwnershipModel)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AssetDefinition::IsCurrency)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(AssetDefinition::IsUserPurchasable)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(AssetDefinition::IsPublic)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(AssetDefinition::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(ColumnDef::new(AssetDefinition::ImageKey).string().null())
                    .col(
                        ColumnDef::new(AssetDefinition::ImageContentType)
                            .string()
                            .null(),
                    )
                    .col(ColumnDef::new(AssetDefinition::Metadata).text().not_null())
                    .col(
                        ColumnDef::new(AssetDefinition::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(AssetDefinition::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AssetDefinition::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum AssetDefinition {
    Table,
    Id,
    Key,
    DisplayName,
    Description,
    AssetKind,
    OwnershipModel,
    IsCurrency,
    IsUserPurchasable,
    IsPublic,
    IsActive,
    ImageKey,
    ImageContentType,
    WeaponKey,
    Rarity,
    Metadata,
    CreatedAt,
    UpdatedAt,
}

pub struct CreateUserStackableAssetTable;

impl MigrationName for CreateUserStackableAssetTable {
    fn name(&self) -> &str {
        "m20260408_000011_create_user_stackable_asset_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateUserStackableAssetTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserStackableAsset::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(UserStackableAsset::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(UserStackableAsset::AssetDefinitionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserStackableAsset::Amount)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserStackableAsset::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .primary_key(
                        sea_orm::sea_query::Index::create()
                            .col(UserStackableAsset::UserId)
                            .col(UserStackableAsset::AssetDefinitionId),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserStackableAsset::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum UserStackableAsset {
    Table,
    UserId,
    AssetDefinitionId,
    Amount,
    UpdatedAt,
}

pub struct CreateUserEntitlementTable;

impl MigrationName for CreateUserEntitlementTable {
    fn name(&self) -> &str {
        "m20260408_000012_create_user_entitlement_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateUserEntitlementTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserEntitlement::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(UserEntitlement::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(UserEntitlement::AssetDefinitionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserEntitlement::GrantedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserEntitlement::GrantedByActor)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserEntitlement::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .primary_key(
                        sea_orm::sea_query::Index::create()
                            .col(UserEntitlement::UserId)
                            .col(UserEntitlement::AssetDefinitionId),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserEntitlement::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum UserEntitlement {
    Table,
    UserId,
    AssetDefinitionId,
    GrantedAt,
    GrantedByActor,
    UpdatedAt,
}

pub struct CreateUserExpirableAssetTable;

impl MigrationName for CreateUserExpirableAssetTable {
    fn name(&self) -> &str {
        "m20260408_000013_create_user_expirable_asset_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateUserExpirableAssetTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserExpirableAsset::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(UserExpirableAsset::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(UserExpirableAsset::AssetDefinitionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserExpirableAsset::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserExpirableAsset::GrantedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserExpirableAsset::LastExtendedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(UserExpirableAsset::GrantedByActor)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserExpirableAsset::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .primary_key(
                        sea_orm::sea_query::Index::create()
                            .col(UserExpirableAsset::UserId)
                            .col(UserExpirableAsset::AssetDefinitionId),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserExpirableAsset::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum UserExpirableAsset {
    Table,
    UserId,
    AssetDefinitionId,
    ExpiresAt,
    GrantedAt,
    LastExtendedAt,
    GrantedByActor,
    UpdatedAt,
}

pub struct CreateInventoryOperationTable;

impl MigrationName for CreateInventoryOperationTable {
    fn name(&self) -> &str {
        "m20260408_000014_create_inventory_operation_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateInventoryOperationTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(InventoryOperation::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(InventoryOperation::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(InventoryOperation::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(InventoryOperation::AssetDefinitionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(InventoryOperation::OwnershipModel)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(InventoryOperation::OperationType)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(InventoryOperation::ActorKind)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(InventoryOperation::ActorUserId)
                            .uuid()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(InventoryOperation::ActorServiceName)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(InventoryOperation::DeltaAmount)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(InventoryOperation::NewAmount)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(InventoryOperation::PreviousExpiresAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(InventoryOperation::NewExpiresAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(InventoryOperation::ReasonCode)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(InventoryOperation::ReasonText)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(InventoryOperation::Metadata)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(InventoryOperation::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(InventoryOperation::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum InventoryOperation {
    Table,
    Id,
    UserId,
    AssetDefinitionId,
    OwnershipModel,
    OperationType,
    ActorKind,
    ActorUserId,
    ActorServiceName,
    DeltaAmount,
    NewAmount,
    PreviousExpiresAt,
    NewExpiresAt,
    ReasonCode,
    ReasonText,
    Metadata,
    CreatedAt,
}

pub struct CreateWalletBalanceTable;

impl MigrationName for CreateWalletBalanceTable {
    fn name(&self) -> &str {
        "m20260408_000015_create_wallet_balance_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateWalletBalanceTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WalletBalance::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(WalletBalance::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(WalletBalance::CurrencyAssetDefinitionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WalletBalance::Balance)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WalletBalance::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .primary_key(
                        sea_orm::sea_query::Index::create()
                            .col(WalletBalance::UserId)
                            .col(WalletBalance::CurrencyAssetDefinitionId),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WalletBalance::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum WalletBalance {
    Table,
    UserId,
    CurrencyAssetDefinitionId,
    Balance,
    UpdatedAt,
}

pub struct CreateWalletTransactionTable;

impl MigrationName for CreateWalletTransactionTable {
    fn name(&self) -> &str {
        "m20260408_000016_create_wallet_transaction_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateWalletTransactionTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WalletTransaction::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WalletTransaction::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WalletTransaction::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(WalletTransaction::CurrencyAssetDefinitionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WalletTransaction::OperationType)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WalletTransaction::ActorKind)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(WalletTransaction::ActorUserId).uuid().null())
                    .col(
                        ColumnDef::new(WalletTransaction::ActorServiceName)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(WalletTransaction::Delta)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WalletTransaction::BalanceAfter)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WalletTransaction::ReasonCode)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(WalletTransaction::ReasonText)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(WalletTransaction::Metadata)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WalletTransaction::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WalletTransaction::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum WalletTransaction {
    Table,
    Id,
    UserId,
    CurrencyAssetDefinitionId,
    OperationType,
    ActorKind,
    ActorUserId,
    ActorServiceName,
    Delta,
    BalanceAfter,
    ReasonCode,
    ReasonText,
    Metadata,
    CreatedAt,
}

pub struct CreateDefaultSkinTable;

impl MigrationName for CreateDefaultSkinTable {
    fn name(&self) -> &str {
        "m20260408_000017_create_default_skin_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateDefaultSkinTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(DefaultSkin::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DefaultSkin::Id)
                            .integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(DefaultSkin::S3Key).string().not_null())
                    .col(ColumnDef::new(DefaultSkin::ContentType).string().not_null())
                    .col(ColumnDef::new(DefaultSkin::UpdatedByUserId).uuid().null())
                    .col(
                        ColumnDef::new(DefaultSkin::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DefaultSkin::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum DefaultSkin {
    Table,
    Id,
    S3Key,
    ContentType,
    UpdatedByUserId,
    UpdatedAt,
}

pub struct CreateServiceTokenTable;

impl MigrationName for CreateServiceTokenTable {
    fn name(&self) -> &str {
        "m20260408_000018_create_service_token_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateServiceTokenTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ServiceToken::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ServiceToken::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ServiceToken::SystemName).string().not_null())
                    .col(
                        ColumnDef::new(ServiceToken::TokenPrefix)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(ServiceToken::TokenHash).string().not_null())
                    .col(
                        ColumnDef::new(ServiceToken::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(ServiceToken::ExpiresAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ServiceToken::LastUsedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(ColumnDef::new(ServiceToken::Description).string().null())
                    .col(
                        ColumnDef::new(ServiceToken::CreatedByUserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ServiceToken::RotatedFromId).uuid().null())
                    .col(
                        ColumnDef::new(ServiceToken::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ServiceToken::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ServiceToken::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ServiceToken {
    Table,
    Id,
    SystemName,
    TokenPrefix,
    TokenHash,
    IsActive,
    ExpiresAt,
    LastUsedAt,
    Description,
    CreatedByUserId,
    RotatedFromId,
    CreatedAt,
    UpdatedAt,
}

pub struct CreateServiceTokenAuditTable;

impl MigrationName for CreateServiceTokenAuditTable {
    fn name(&self) -> &str {
        "m20260408_000019_create_service_token_audit_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateServiceTokenAuditTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ServiceTokenAudit::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ServiceTokenAudit::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ServiceTokenAudit::ServiceTokenId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ServiceTokenAudit::Action)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ServiceTokenAudit::ActorUserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ServiceTokenAudit::Reason).string().null())
                    .col(ColumnDef::new(ServiceTokenAudit::Metadata).text().null())
                    .col(
                        ColumnDef::new(ServiceTokenAudit::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ServiceTokenAudit::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ServiceTokenAudit {
    Table,
    Id,
    ServiceTokenId,
    Action,
    ActorUserId,
    Reason,
    Metadata,
    CreatedAt,
}

pub struct CreateLootboxDefinitionTable;

impl MigrationName for CreateLootboxDefinitionTable {
    fn name(&self) -> &str {
        "m20260409_000020_create_lootbox_definition_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateLootboxDefinitionTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LootboxDefinition::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LootboxDefinition::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(LootboxDefinition::AssetDefinitionId)
                            .uuid()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(LootboxDefinition::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(LootboxDefinition::Metadata)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LootboxDefinition::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(LootboxDefinition::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(LootboxDefinition::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum LootboxDefinition {
    Table,
    Id,
    AssetDefinitionId,
    IsActive,
    Metadata,
    CreatedAt,
    UpdatedAt,
}

pub struct CreateLootboxDropDefinitionTable;

impl MigrationName for CreateLootboxDropDefinitionTable {
    fn name(&self) -> &str {
        "m20260409_000021_create_lootbox_drop_definition_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateLootboxDropDefinitionTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LootboxDropDefinition::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LootboxDropDefinition::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(LootboxDropDefinition::LootboxDefinitionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LootboxDropDefinition::RewardAssetDefinitionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LootboxDropDefinition::StackableAmount)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LootboxDropDefinition::ExpirableDurationSeconds)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LootboxDropDefinition::DuplicateCompensationAmount)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LootboxDropDefinition::Weight)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LootboxDropDefinition::TitleI18n)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LootboxDropDefinition::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(LootboxDropDefinition::SortOrder)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(LootboxDropDefinition::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(LootboxDropDefinition::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(LootboxDropDefinition::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum LootboxDropDefinition {
    Table,
    Id,
    LootboxDefinitionId,
    RewardAssetDefinitionId,
    StackableAmount,
    ExpirableDurationSeconds,
    DuplicateCompensationAmount,
    Weight,
    TitleI18n,
    IsActive,
    SortOrder,
    CreatedAt,
    UpdatedAt,
}

pub struct CreateLootboxOpenOperationTable;

impl MigrationName for CreateLootboxOpenOperationTable {
    fn name(&self) -> &str {
        "m20260409_000022_create_lootbox_open_operation_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateLootboxOpenOperationTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LootboxOpenOperation::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LootboxOpenOperation::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::LootboxDefinitionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::LootboxAssetDefinitionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::SelectedDropDefinitionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::RewardAssetDefinitionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::RewardOwnershipModel)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::RewardAmount)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::RewardDurationSeconds)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::RewardExpiresAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::RewardTitle)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::GrantedAssetDefinitionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::GrantedOwnershipModel)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::GrantedAmount)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::GrantedDurationSeconds)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::GrantedExpiresAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::GrantedTitle)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::WasCompensated)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(LootboxOpenOperation::Locale).string().null())
                    .col(
                        ColumnDef::new(LootboxOpenOperation::ActorKind)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::ActorUserId)
                            .uuid()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::ActorServiceName)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::FeedLength)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::WinnerIndex)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::FeedJson)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LootboxOpenOperation::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(LootboxOpenOperation::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum LootboxOpenOperation {
    Table,
    Id,
    UserId,
    LootboxDefinitionId,
    LootboxAssetDefinitionId,
    SelectedDropDefinitionId,
    RewardAssetDefinitionId,
    RewardOwnershipModel,
    RewardAmount,
    RewardDurationSeconds,
    RewardExpiresAt,
    RewardTitle,
    GrantedAssetDefinitionId,
    GrantedOwnershipModel,
    GrantedAmount,
    GrantedDurationSeconds,
    GrantedExpiresAt,
    GrantedTitle,
    WasCompensated,
    Locale,
    ActorKind,
    ActorUserId,
    ActorServiceName,
    FeedLength,
    WinnerIndex,
    FeedJson,
    CreatedAt,
}

pub struct CreateShopProductTable;

pub struct AddLootboxRewardCompensationColumns;

impl MigrationName for AddLootboxRewardCompensationColumns {
    fn name(&self) -> &str {
        "m20260417_000036_add_lootbox_reward_compensation_columns"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for AddLootboxRewardCompensationColumns {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let statements = match backend {
            DatabaseBackend::Postgres | DatabaseBackend::Sqlite => vec![
                r#"ALTER TABLE "lootbox_drop_definition" ADD COLUMN "duplicate_compensation_amount" bigint NULL"#,
                r#"ALTER TABLE "lootbox_open_operation" ADD COLUMN "granted_asset_definition_id" uuid NULL"#,
                r#"ALTER TABLE "lootbox_open_operation" ADD COLUMN "granted_ownership_model" varchar NULL"#,
                r#"ALTER TABLE "lootbox_open_operation" ADD COLUMN "granted_amount" bigint NULL"#,
                r#"ALTER TABLE "lootbox_open_operation" ADD COLUMN "granted_duration_seconds" bigint NULL"#,
                r#"ALTER TABLE "lootbox_open_operation" ADD COLUMN "granted_expires_at" timestamptz NULL"#,
                r#"ALTER TABLE "lootbox_open_operation" ADD COLUMN "granted_title" varchar NULL"#,
                r#"ALTER TABLE "lootbox_open_operation" ADD COLUMN "was_compensated" boolean NOT NULL DEFAULT false"#,
                r#"UPDATE "lootbox_open_operation" SET "granted_asset_definition_id" = "reward_asset_definition_id" WHERE "granted_asset_definition_id" IS NULL"#,
                r#"UPDATE "lootbox_open_operation" SET "granted_ownership_model" = "reward_ownership_model" WHERE "granted_ownership_model" IS NULL"#,
                r#"UPDATE "lootbox_open_operation" SET "granted_amount" = "reward_amount" WHERE "granted_amount" IS NULL"#,
                r#"UPDATE "lootbox_open_operation" SET "granted_duration_seconds" = "reward_duration_seconds" WHERE "granted_duration_seconds" IS NULL"#,
                r#"UPDATE "lootbox_open_operation" SET "granted_expires_at" = "reward_expires_at" WHERE "granted_expires_at" IS NULL"#,
                r#"UPDATE "lootbox_open_operation" SET "granted_title" = "reward_title" WHERE "granted_title" IS NULL"#,
            ],
            _ => return Ok(()),
        };

        for sql in statements {
            match manager
                .get_connection()
                .execute(Statement::from_string(backend, sql.to_string()))
                .await
            {
                Ok(_) => {}
                Err(error) if is_duplicate_column_error(&error) => {}
                Err(error) => return Err(error),
            }
        }

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

impl MigrationName for CreateShopProductTable {
    fn name(&self) -> &str {
        "m20260412_000024_create_shop_product_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateShopProductTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ShopProduct::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ShopProduct::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ShopProduct::Key)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(ShopProduct::AssetDefinitionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ShopProduct::PriceRub)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ShopProduct::StackableAmount)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ShopProduct::ExpirableDurationSeconds)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ShopProduct::MaxPerPurchase)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ShopProduct::MaxOwnedAmount)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ShopProduct::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(ShopProduct::IsPublic)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(ShopProduct::SortOrder)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(ShopProduct::StartsAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ShopProduct::EndsAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(ColumnDef::new(ShopProduct::Metadata).text().not_null())
                    .col(
                        ColumnDef::new(ShopProduct::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ShopProduct::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ShopProduct::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ShopProduct {
    Table,
    Id,
    Key,
    AssetDefinitionId,
    PriceRub,
    StackableAmount,
    ExpirableDurationSeconds,
    MaxPerPurchase,
    MaxOwnedAmount,
    IsActive,
    IsPublic,
    SortOrder,
    StartsAt,
    EndsAt,
    Metadata,
    CreatedAt,
    UpdatedAt,
}

pub struct CreateShopProductLocaleTable;

impl MigrationName for CreateShopProductLocaleTable {
    fn name(&self) -> &str {
        "m20260412_000025_create_shop_product_locale_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateShopProductLocaleTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ShopProductLocale::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ShopProductLocale::ProductId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ShopProductLocale::Locale)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ShopProductLocale::Name).string().not_null())
                    .col(
                        ColumnDef::new(ShopProductLocale::Description)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ShopProductLocale::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ShopProductLocale::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .primary_key(
                        sea_orm::sea_query::Index::create()
                            .col(ShopProductLocale::ProductId)
                            .col(ShopProductLocale::Locale),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ShopProductLocale::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ShopProductLocale {
    Table,
    ProductId,
    Locale,
    Name,
    Description,
    CreatedAt,
    UpdatedAt,
}

pub struct CreateShopOrderTable;

impl MigrationName for CreateShopOrderTable {
    fn name(&self) -> &str {
        "m20260412_000026_create_shop_order_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateShopOrderTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ShopOrder::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ShopOrder::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ShopOrder::UserId).uuid().not_null())
                    .col(ColumnDef::new(ShopOrder::ProductId).uuid().not_null())
                    .col(ColumnDef::new(ShopOrder::ProductKey).string().not_null())
                    .col(ColumnDef::new(ShopOrder::ProductLocale).string().null())
                    .col(ColumnDef::new(ShopOrder::ProductName).string().not_null())
                    .col(
                        ColumnDef::new(ShopOrder::ProductDescription)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ShopOrder::AssetDefinitionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ShopOrder::AssetKey).string().not_null())
                    .col(
                        ColumnDef::new(ShopOrder::OwnershipModel)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ShopOrder::Quantity).big_integer().not_null())
                    .col(
                        ColumnDef::new(ShopOrder::UnitPriceRub)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ShopOrder::TotalPriceRub)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ShopOrder::StackableAmountPerUnit)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ShopOrder::ExpirableDurationSecondsPerUnit)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ShopOrder::MaxOwnedAmountSnapshot)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ShopOrder::PaymentProvider)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ShopOrder::Status).string().not_null())
                    .col(ColumnDef::new(ShopOrder::FailureProblem).string().null())
                    .col(
                        ColumnDef::new(ShopOrder::PaymentExpiresAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ShopOrder::PaidAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ShopOrder::FulfilledAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(ColumnDef::new(ShopOrder::Metadata).text().not_null())
                    .col(
                        ColumnDef::new(ShopOrder::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ShopOrder::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ShopOrder::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ShopOrder {
    Table,
    Id,
    UserId,
    ProductId,
    ProductKey,
    ProductLocale,
    ProductName,
    ProductDescription,
    AssetDefinitionId,
    AssetKey,
    OwnershipModel,
    Quantity,
    UnitPriceRub,
    TotalPriceRub,
    StackableAmountPerUnit,
    ExpirableDurationSecondsPerUnit,
    MaxOwnedAmountSnapshot,
    PaymentProvider,
    Status,
    FailureProblem,
    PaymentExpiresAt,
    PaidAt,
    FulfilledAt,
    Metadata,
    CreatedAt,
    UpdatedAt,
}

pub struct CreateShopPaymentAttemptTable;

impl MigrationName for CreateShopPaymentAttemptTable {
    fn name(&self) -> &str {
        "m20260412_000027_create_shop_payment_attempt_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateShopPaymentAttemptTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ShopPaymentAttempt::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ShopPaymentAttempt::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ShopPaymentAttempt::OrderId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ShopPaymentAttempt::Provider)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ShopPaymentAttempt::ProviderPaymentId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ShopPaymentAttempt::CheckoutToken)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ShopPaymentAttempt::CheckoutUrl)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ShopPaymentAttempt::Status)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ShopPaymentAttempt::RequestPayload)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ShopPaymentAttempt::ResponsePayload)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ShopPaymentAttempt::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ShopPaymentAttempt::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ShopPaymentAttempt::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ShopPaymentAttempt {
    Table,
    Id,
    OrderId,
    Provider,
    ProviderPaymentId,
    CheckoutToken,
    CheckoutUrl,
    Status,
    RequestPayload,
    ResponsePayload,
    CreatedAt,
    UpdatedAt,
}

pub struct AddShopOrderPaymentLifecycleColumns;

impl MigrationName for AddShopOrderPaymentLifecycleColumns {
    fn name(&self) -> &str {
        "m20260413_000028_add_shop_order_payment_lifecycle_columns"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for AddShopOrderPaymentLifecycleColumns {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let statements = match backend {
            DatabaseBackend::Postgres | DatabaseBackend::Sqlite => vec![
                r#"ALTER TABLE "shop_order" ADD COLUMN "payment_expires_at" timestamptz NULL"#,
                r#"CREATE INDEX IF NOT EXISTS "idx_shop_order_status_payment_expires_at" ON "shop_order" ("status", "payment_expires_at")"#,
            ],
            _ => return Ok(()),
        };

        for sql in statements {
            match manager
                .get_connection()
                .execute(Statement::from_string(backend, sql.to_string()))
                .await
            {
                Ok(_) => {}
                Err(error)
                    if is_duplicate_column_error(&error) || is_duplicate_index_error(&error) => {}
                Err(error) => return Err(error),
            }
        }

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

pub struct AddShopPaymentAttemptProviderIndex;

impl MigrationName for AddShopPaymentAttemptProviderIndex {
    fn name(&self) -> &str {
        "m20260413_000029_add_shop_payment_attempt_provider_index"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for AddShopPaymentAttemptProviderIndex {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let sql = match backend {
            DatabaseBackend::Postgres | DatabaseBackend::Sqlite => {
                r#"CREATE UNIQUE INDEX IF NOT EXISTS "idx_shop_payment_attempt_provider_payment_id" ON "shop_payment_attempt" ("provider", "provider_payment_id")"#
            }
            _ => return Ok(()),
        };

        match manager
            .get_connection()
            .execute(Statement::from_string(backend, sql.to_string()))
            .await
        {
            Ok(_) => Ok(()),
            Err(error) if is_duplicate_index_error(&error) => Ok(()),
            Err(error) => Err(error),
        }
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

fn is_duplicate_index_error(error: &DbErr) -> bool {
    let error_text = error.to_string().to_lowercase();
    error_text.contains("already exists")
        || error_text.contains("duplicate")
        || error_text.contains("exists")
}

pub struct CreateDiscordBroadcastTable;

impl MigrationName for CreateDiscordBroadcastTable {
    fn name(&self) -> &str {
        "m20260413_000030_create_discord_broadcast_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateDiscordBroadcastTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(DiscordBroadcast::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DiscordBroadcast::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(DiscordBroadcast::RequestedByUserId)
                            .uuid()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DiscordBroadcast::TemplateKey)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(DiscordBroadcast::Message).text().not_null())
                    .col(ColumnDef::new(DiscordBroadcast::Status).string().not_null())
                    .col(
                        ColumnDef::new(DiscordBroadcast::TotalCount)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DiscordBroadcast::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(DiscordBroadcast::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(DiscordBroadcast::StartedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DiscordBroadcast::FinishedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DiscordBroadcast::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum DiscordBroadcast {
    Table,
    Id,
    RequestedByUserId,
    TemplateKey,
    Message,
    Status,
    TotalCount,
    CreatedAt,
    UpdatedAt,
    StartedAt,
    FinishedAt,
}

pub struct CreateDiscordDeliveryTable;

impl MigrationName for CreateDiscordDeliveryTable {
    fn name(&self) -> &str {
        "m20260413_000031_create_discord_delivery_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateDiscordDeliveryTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(DiscordDelivery::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DiscordDelivery::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(DiscordDelivery::BroadcastId).uuid().null())
                    .col(ColumnDef::new(DiscordDelivery::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(DiscordDelivery::RequestedByUserId)
                            .uuid()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DiscordDelivery::TemplateKey)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(DiscordDelivery::Message).text().not_null())
                    .col(ColumnDef::new(DiscordDelivery::Status).string().not_null())
                    .col(ColumnDef::new(DiscordDelivery::ErrorMessage).text().null())
                    .col(
                        ColumnDef::new(DiscordDelivery::DiscordChannelId)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DiscordDelivery::DiscordMessageId)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DiscordDelivery::AttemptCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(DiscordDelivery::Metadata).text().not_null())
                    .col(
                        ColumnDef::new(DiscordDelivery::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(DiscordDelivery::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(DiscordDelivery::LastAttemptAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DiscordDelivery::DeliveredAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DiscordDelivery::FinishedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        let backend = manager.get_database_backend();
        for sql in [
            r#"CREATE INDEX IF NOT EXISTS "idx_discord_delivery_status_created_at" ON "discord_delivery" ("status", "created_at")"#,
            r#"CREATE INDEX IF NOT EXISTS "idx_discord_delivery_broadcast_id" ON "discord_delivery" ("broadcast_id")"#,
        ] {
            match manager
                .get_connection()
                .execute(Statement::from_string(backend, sql.to_string()))
                .await
            {
                Ok(_) => {}
                Err(error) if is_duplicate_index_error(&error) => {}
                Err(error) => return Err(error),
            }
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DiscordDelivery::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum DiscordDelivery {
    Table,
    Id,
    BroadcastId,
    UserId,
    RequestedByUserId,
    TemplateKey,
    Message,
    Status,
    ErrorMessage,
    DiscordChannelId,
    DiscordMessageId,
    AttemptCount,
    Metadata,
    CreatedAt,
    UpdatedAt,
    LastAttemptAt,
    DeliveredAt,
    FinishedAt,
}

pub struct CreateEmailDeliveryTable;

impl MigrationName for CreateEmailDeliveryTable {
    fn name(&self) -> &str {
        "m20260419_000036_create_email_delivery_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateEmailDeliveryTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(EmailDelivery::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(EmailDelivery::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(EmailDelivery::UserId).uuid().null())
                    .col(
                        ColumnDef::new(EmailDelivery::RequestedByUserId)
                            .uuid()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(EmailDelivery::RecipientEmail)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(EmailDelivery::TemplateKey)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(EmailDelivery::Subject).text().not_null())
                    .col(ColumnDef::new(EmailDelivery::HtmlBody).text().not_null())
                    .col(ColumnDef::new(EmailDelivery::Status).string().not_null())
                    .col(ColumnDef::new(EmailDelivery::ErrorMessage).text().null())
                    .col(
                        ColumnDef::new(EmailDelivery::ProviderMessageId)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(EmailDelivery::AttemptCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(EmailDelivery::Metadata).text().not_null())
                    .col(ColumnDef::new(EmailDelivery::AttachmentUrl).text().null())
                    .col(
                        ColumnDef::new(EmailDelivery::AttachmentFilename)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(EmailDelivery::AttachmentContentType)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(EmailDelivery::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(EmailDelivery::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(EmailDelivery::LastAttemptAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(EmailDelivery::DeliveredAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(EmailDelivery::FinishedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        let backend = manager.get_database_backend();
        for sql in [
            r#"CREATE INDEX IF NOT EXISTS "idx_email_delivery_status_created_at" ON "email_delivery" ("status", "created_at")"#,
            r#"CREATE INDEX IF NOT EXISTS "idx_email_delivery_user_id" ON "email_delivery" ("user_id")"#,
        ] {
            match manager
                .get_connection()
                .execute(Statement::from_string(backend, sql.to_string()))
                .await
            {
                Ok(_) => {}
                Err(error) if is_duplicate_index_error(&error) => {}
                Err(error) => return Err(error),
            }
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(EmailDelivery::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum EmailDelivery {
    Table,
    Id,
    UserId,
    RequestedByUserId,
    RecipientEmail,
    TemplateKey,
    Subject,
    HtmlBody,
    Status,
    ErrorMessage,
    ProviderMessageId,
    AttemptCount,
    Metadata,
    AttachmentUrl,
    AttachmentFilename,
    AttachmentContentType,
    CreatedAt,
    UpdatedAt,
    LastAttemptAt,
    DeliveredAt,
    FinishedAt,
}

pub struct AddAssetDefinitionGunskinColumns;

impl MigrationName for AddAssetDefinitionGunskinColumns {
    fn name(&self) -> &str {
        "m20260413_000032_add_asset_definition_gunskin_columns"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for AddAssetDefinitionGunskinColumns {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let statements = match backend {
            DatabaseBackend::Postgres | DatabaseBackend::Sqlite => vec![
                r#"ALTER TABLE "asset_definition" ADD COLUMN "weapon_key" varchar NULL"#,
                r#"ALTER TABLE "asset_definition" ADD COLUMN "rarity" varchar NULL"#,
                r#"ALTER TABLE "asset_definition" ADD COLUMN "image_key" varchar NULL"#,
                r#"ALTER TABLE "asset_definition" ADD COLUMN "image_content_type" varchar NULL"#,
                r#"CREATE INDEX IF NOT EXISTS "idx_asset_definition_weapon_key" ON "asset_definition" ("weapon_key")"#,
            ],
            _ => return Ok(()),
        };

        for sql in statements {
            match manager
                .get_connection()
                .execute(Statement::from_string(backend, sql.to_string()))
                .await
            {
                Ok(_) => {}
                Err(error)
                    if is_duplicate_column_error(&error) || is_duplicate_index_error(&error) => {}
                Err(error) => return Err(error),
            }
        }

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

pub struct CreateUserSelectedGunskinTable;

impl MigrationName for CreateUserSelectedGunskinTable {
    fn name(&self) -> &str {
        "m20260413_000033_create_user_selected_gunskin_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateUserSelectedGunskinTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserSelectedGunskin::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserSelectedGunskin::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserSelectedGunskin::WeaponKey)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserSelectedGunskin::AssetDefinitionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserSelectedGunskin::SelectedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(UserSelectedGunskin::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .primary_key(
                        sea_orm::sea_query::Index::create()
                            .col(UserSelectedGunskin::UserId)
                            .col(UserSelectedGunskin::WeaponKey),
                    )
                    .to_owned(),
            )
            .await?;

        let backend = manager.get_database_backend();
        for sql in [
            r#"CREATE INDEX IF NOT EXISTS "idx_user_selected_gunskin_asset_definition_id" ON "user_selected_gunskin" ("asset_definition_id")"#,
            r#"CREATE INDEX IF NOT EXISTS "idx_user_selected_gunskin_weapon_key" ON "user_selected_gunskin" ("weapon_key")"#,
        ] {
            match manager
                .get_connection()
                .execute(Statement::from_string(backend, sql.to_string()))
                .await
            {
                Ok(_) => {}
                Err(error) if is_duplicate_index_error(&error) => {}
                Err(error) => return Err(error),
            }
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserSelectedGunskin::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum UserSelectedGunskin {
    Table,
    UserId,
    WeaponKey,
    AssetDefinitionId,
    SelectedAt,
    UpdatedAt,
}

pub struct CreateAppKvTable;

impl MigrationName for CreateAppKvTable {
    fn name(&self) -> &str {
        "m20260417_000034_create_app_kv_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateAppKvTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AppKv::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(AppKv::Key).string().not_null().primary_key())
                    .col(ColumnDef::new(AppKv::Value).text().not_null())
                    .col(
                        ColumnDef::new(AppKv::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(AppKv::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AppKv::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum AppKv {
    Table,
    Key,
    Value,
    CreatedAt,
    UpdatedAt,
}

pub struct CreateShopReceiptTable;

impl MigrationName for CreateShopReceiptTable {
    fn name(&self) -> &str {
        "m20260417_000035_create_shop_receipt_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateShopReceiptTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ShopReceipt::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ShopReceipt::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ShopReceipt::OrderId).uuid().not_null())
                    .col(ColumnDef::new(ShopReceipt::Provider).string().not_null())
                    .col(ColumnDef::new(ShopReceipt::Status).string().not_null())
                    .col(ColumnDef::new(ShopReceipt::ReceiptUuid).string().null())
                    .col(ColumnDef::new(ShopReceipt::JsonUrl).string().null())
                    .col(ColumnDef::new(ShopReceipt::PrintUrl).string().null())
                    .col(
                        ColumnDef::new(ShopReceipt::AttemptCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(ShopReceipt::FirstAttemptAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ShopReceipt::LastAttemptAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ShopReceipt::NextAttemptAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ShopReceipt::DeadlineAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ShopReceipt::LockedUntil)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(ColumnDef::new(ShopReceipt::FailureProblem).string().null())
                    .col(
                        ColumnDef::new(ShopReceipt::RequestPayload)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ShopReceipt::ResponsePayload)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ShopReceipt::CompletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ShopReceipt::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ShopReceipt::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        let backend = manager.get_database_backend();
        for sql in [
            r#"CREATE UNIQUE INDEX IF NOT EXISTS "idx_shop_receipt_order_id" ON "shop_receipt" ("order_id")"#,
            r#"CREATE INDEX IF NOT EXISTS "idx_shop_receipt_status_next_attempt_at" ON "shop_receipt" ("status", "next_attempt_at")"#,
            r#"CREATE INDEX IF NOT EXISTS "idx_shop_receipt_locked_until" ON "shop_receipt" ("locked_until")"#,
        ] {
            match manager
                .get_connection()
                .execute(Statement::from_string(backend, sql.to_string()))
                .await
            {
                Ok(_) => {}
                Err(error) if is_duplicate_index_error(&error) => {}
                Err(error) => return Err(error),
            }
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ShopReceipt::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ShopReceipt {
    Table,
    Id,
    OrderId,
    Provider,
    Status,
    ReceiptUuid,
    JsonUrl,
    PrintUrl,
    AttemptCount,
    FirstAttemptAt,
    LastAttemptAt,
    NextAttemptAt,
    DeadlineAt,
    LockedUntil,
    FailureProblem,
    RequestPayload,
    ResponsePayload,
    CompletedAt,
    CreatedAt,
    UpdatedAt,
}
