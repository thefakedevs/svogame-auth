use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "metric_ingestion")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub game_id: Uuid,
    pub content_sha256: String,
    pub status: String,
    pub byte_count: i64,
    pub event_count: i64,
    pub warnings: String,
    pub created_at: DateTimeUtc,
    pub completed_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
