use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, DeriveRelation, EnumIter};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "inventory_operation")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub user_id: Uuid,
    pub asset_definition_id: Uuid,
    pub ownership_model: String,
    pub operation_type: String,
    pub actor_kind: String,
    pub actor_user_id: Option<Uuid>,
    pub actor_service_name: Option<String>,
    pub delta_amount: Option<i64>,
    pub new_amount: Option<i64>,
    pub previous_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub new_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub reason_code: Option<String>,
    pub reason_text: Option<String>,
    pub metadata: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
