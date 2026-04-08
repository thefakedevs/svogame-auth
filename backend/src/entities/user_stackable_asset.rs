use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, DeriveRelation, EnumIter};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user_stackable_asset")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub asset_definition_id: Uuid,
    pub amount: i64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
