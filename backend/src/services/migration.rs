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

