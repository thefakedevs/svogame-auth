use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "lootbox_open_operation")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub user_id: Uuid,
    pub lootbox_definition_id: Uuid,
    pub lootbox_asset_definition_id: Uuid,
    pub selected_drop_definition_id: Uuid,
    pub reward_asset_definition_id: Uuid,
    pub reward_ownership_model: String,
    pub reward_amount: Option<i64>,
    pub reward_duration_seconds: Option<i64>,
    pub reward_expires_at: Option<DateTimeUtc>,
    pub reward_title: String,
    pub locale: Option<String>,
    pub actor_kind: String,
    pub actor_user_id: Option<Uuid>,
    pub actor_service_name: Option<String>,
    pub feed_length: i32,
    pub winner_index: i32,
    pub feed_json: String,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
