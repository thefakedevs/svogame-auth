use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "lootbox_drop_definition")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub lootbox_definition_id: Uuid,
    pub reward_asset_definition_id: Uuid,
    pub stackable_amount: Option<i64>,
    pub expirable_duration_seconds: Option<i64>,
    pub duplicate_compensation_amount: Option<i64>,
    pub weight: i64,
    pub title_i18n: String,
    pub is_active: bool,
    pub sort_order: i32,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
