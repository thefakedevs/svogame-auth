use sea_orm::prelude::Expr;
use sea_orm::sea_query::{ColumnDef, Table};
use sea_orm::{DbErr, DeriveIden};
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
    LastLoginAt,
    CreatedAt,
}

