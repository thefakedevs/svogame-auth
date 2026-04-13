use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, DeriveRelation, EnumIter};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "shop_order")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub product_id: Uuid,
    pub product_key: String,
    pub product_locale: Option<String>,
    pub product_name: String,
    pub product_description: Option<String>,
    pub asset_definition_id: Uuid,
    pub asset_key: String,
    pub ownership_model: String,
    pub quantity: i64,
    pub unit_price_rub: i64,
    pub total_price_rub: i64,
    pub stackable_amount_per_unit: Option<i64>,
    pub expirable_duration_seconds_per_unit: Option<i64>,
    pub max_owned_amount_snapshot: Option<i64>,
    pub payment_provider: String,
    pub status: String,
    pub failure_problem: Option<String>,
    pub payment_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub paid_at: Option<chrono::DateTime<chrono::Utc>>,
    pub fulfilled_at: Option<chrono::DateTime<chrono::Utc>>,
    pub metadata: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
