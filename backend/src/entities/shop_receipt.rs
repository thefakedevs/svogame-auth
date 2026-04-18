use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, DeriveRelation, EnumIter};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "shop_receipt")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub order_id: Uuid,
    pub provider: String,
    pub status: String,
    pub receipt_uuid: Option<String>,
    pub json_url: Option<String>,
    pub print_url: Option<String>,
    pub attempt_count: i32,
    pub first_attempt_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_attempt_at: Option<chrono::DateTime<chrono::Utc>>,
    pub next_attempt_at: Option<chrono::DateTime<chrono::Utc>>,
    pub deadline_at: Option<chrono::DateTime<chrono::Utc>>,
    pub locked_until: Option<chrono::DateTime<chrono::Utc>>,
    pub failure_problem: Option<String>,
    pub request_payload: String,
    pub response_payload: String,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
