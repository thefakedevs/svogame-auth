use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "metric_player")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub player_id: Uuid,
    pub last_nickname: String,
    pub first_seen_at: DateTimeUtc,
    pub last_seen_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
