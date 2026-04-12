use crate::app::config::DatabaseConfig;
use crate::services::migration::{
    AddAuthRayRegistrationColumns, AddUserDeactivationReasonColumn, AddUserSquadIdColumn,
    AddUserSuperuserColumn,
    CreateAssetDefinitionTable, CreateAuditLogTable, CreateAuthRayTable, CreateDefaultSkinTable,
    CreateInventoryOperationTable, CreateLootboxDefinitionTable, CreateLootboxDropDefinitionTable,
    CreateLootboxOpenOperationTable, CreateServiceTokenAuditTable, CreateServiceTokenTable,
    CreateSquadInviteTable, CreateSquadTable, CreateUserEntitlementTable,
    CreateUserExpirableAssetTable, CreateUserRestrictionTable, CreateUserStackableAssetTable,
    CreateUserTable, CreateWalletBalanceTable, CreateWalletTransactionTable,
};
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
    AddAuthRayRegistrationColumns.up(&schema_manager).await?;
    CreateUserTable.up(&schema_manager).await?;
    AddUserSuperuserColumn.up(&schema_manager).await?;
    AddUserDeactivationReasonColumn.up(&schema_manager).await?;
    AddUserSquadIdColumn.up(&schema_manager).await?;
    CreateAuditLogTable.up(&schema_manager).await?;
    CreateSquadTable.up(&schema_manager).await?;
    CreateSquadInviteTable.up(&schema_manager).await?;
    CreateUserRestrictionTable.up(&schema_manager).await?;
    CreateAssetDefinitionTable.up(&schema_manager).await?;
    CreateUserStackableAssetTable.up(&schema_manager).await?;
    CreateUserEntitlementTable.up(&schema_manager).await?;
    CreateUserExpirableAssetTable.up(&schema_manager).await?;
    CreateInventoryOperationTable.up(&schema_manager).await?;
    CreateWalletBalanceTable.up(&schema_manager).await?;
    CreateWalletTransactionTable.up(&schema_manager).await?;
    CreateDefaultSkinTable.up(&schema_manager).await?;
    CreateServiceTokenTable.up(&schema_manager).await?;
    CreateServiceTokenAuditTable.up(&schema_manager).await?;
    CreateLootboxDefinitionTable.up(&schema_manager).await?;
    CreateLootboxDropDefinitionTable.up(&schema_manager).await?;
    CreateLootboxOpenOperationTable.up(&schema_manager).await?;
    Ok(())
}
