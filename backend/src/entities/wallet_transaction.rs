use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, DeriveRelation, EnumIter};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "wallet_transaction")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub user_id: Uuid,
    pub currency_asset_definition_id: Uuid,
    pub operation_type: String,
    pub actor_kind: String,
    pub actor_user_id: Option<Uuid>,
    pub actor_service_name: Option<String>,
    pub delta: i64,
    pub balance_after: i64,
    pub reason_code: Option<String>,
    pub reason_text: Option<String>,
    pub metadata: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
