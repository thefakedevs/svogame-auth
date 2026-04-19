use crate::app::config::DatabaseConfig;
use crate::services::migration::{
    AddAssetDefinitionGunskinColumns, AddAuthRayRegistrationColumns,
    AddLootboxRewardCompensationColumns, AddShopOrderPaymentLifecycleColumns,
    AddShopPaymentAttemptProviderIndex, AddUserDeactivationReasonColumn, AddUserSquadIdColumn,
    AddUserSuperuserColumn, CreateAppKvTable, CreateAssetDefinitionTable, CreateAuditLogTable,
    CreateAuthRayTable, CreateDefaultSkinTable, CreateDiscordBroadcastTable,
    CreateDiscordDeliveryTable, CreateEmailDeliveryTable, CreateInventoryOperationTable,
    CreateLootboxDefinitionTable, CreateLootboxDropDefinitionTable,
    CreateLootboxOpenOperationTable, CreateServiceTokenAuditTable, CreateServiceTokenTable,
    CreateShopOrderTable, CreateShopPaymentAttemptTable, CreateShopProductLocaleTable,
    CreateShopProductTable, CreateShopReceiptTable, CreateSquadInviteTable, CreateSquadTable,
    CreateUserEntitlementTable, CreateUserExpirableAssetTable, CreateUserRestrictionTable,
    CreateUserSelectedGunskinTable, CreateUserStackableAssetTable, CreateUserTable,
    CreateWalletBalanceTable, CreateWalletTransactionTable,
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
    CreateAppKvTable.up(&schema_manager).await?;
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
    AddLootboxRewardCompensationColumns
        .up(&schema_manager)
        .await?;
    CreateShopProductTable.up(&schema_manager).await?;
    CreateShopProductLocaleTable.up(&schema_manager).await?;
    CreateShopOrderTable.up(&schema_manager).await?;
    CreateShopPaymentAttemptTable.up(&schema_manager).await?;
    CreateShopReceiptTable.up(&schema_manager).await?;
    CreateDiscordBroadcastTable.up(&schema_manager).await?;
    CreateDiscordDeliveryTable.up(&schema_manager).await?;
    CreateEmailDeliveryTable.up(&schema_manager).await?;
    AddShopOrderPaymentLifecycleColumns
        .up(&schema_manager)
        .await?;
    AddShopPaymentAttemptProviderIndex
        .up(&schema_manager)
        .await?;
    AddAssetDefinitionGunskinColumns.up(&schema_manager).await?;
    CreateUserSelectedGunskinTable.up(&schema_manager).await?;
    Ok(())
}
