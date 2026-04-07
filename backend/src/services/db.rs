use crate::services::migration::{
    AddUserDeactivationReasonColumn, AddUserSquadIdColumn, AddUserSuperuserColumn,
    CreateAuditLogTable, CreateAuthRayTable, CreateSquadInviteTable, CreateSquadTable,
    CreateUserRestrictionTable, CreateUserTable,
};
use crate::app::config::DatabaseConfig;
use anyhow::Result;
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::prelude::*;
use sea_orm_migration::{MigrationTrait, SchemaManager};

pub async fn connect_db(config: &DatabaseConfig) -> Result<DatabaseConnection> {
    let db = Database::connect(config.db_url.as_str()).await?;
    Ok(db)
}

pub async fn run_migrations(db: &DatabaseConnection) -> Result<()> {
    let schema_manager = SchemaManager::new(db);
    CreateAuthRayTable.up(&schema_manager).await?;
    CreateUserTable.up(&schema_manager).await?;
    AddUserSuperuserColumn.up(&schema_manager).await?;
    AddUserDeactivationReasonColumn.up(&schema_manager).await?;
    AddUserSquadIdColumn.up(&schema_manager).await?;
    CreateAuditLogTable.up(&schema_manager).await?;
    CreateSquadTable.up(&schema_manager).await?;
    CreateSquadInviteTable.up(&schema_manager).await?;
    CreateUserRestrictionTable.up(&schema_manager).await?;
    Ok(())
}


