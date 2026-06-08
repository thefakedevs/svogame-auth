use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, DeriveRelation, EnumIter};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "referral_registration")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub user_id: Uuid,
    pub code_snapshot: String,
    pub campaign_title_snapshot: String,
    pub content_creator_user_id_snapshot: Option<Uuid>,
    pub source: String,
    pub reward_status: String,
    pub reward_error: Option<String>,
    pub metadata: String,
    pub registered_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
