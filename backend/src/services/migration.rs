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
                    .col(ColumnDef::new(AuthRay::PowComplexity).tiny_unsigned().not_null())
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
enum AuthRay {
    Table,
    Id,
    PowPrefix,
    PowComplexity,
    DeliveryMethod,
    DeliveryTarget,
    CreatedAt,
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
                    .col(
                        ColumnDef::new(User::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(User::DiscordId).string().not_null().unique_key())
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
            DatabaseBackend::Postgres => {
                r#"ALTER TABLE "user" ADD COLUMN "squad_id" uuid NULL"#
            }
            DatabaseBackend::Sqlite => {
                r#"ALTER TABLE "user" ADD COLUMN "squad_id" uuid NULL"#
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
                    .col(
                        ColumnDef::new(Squad::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
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
                    .col(ColumnDef::new(UserRestriction::RestrictionKey).string().not_null())
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

