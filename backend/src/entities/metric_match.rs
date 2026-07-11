use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "metric_match")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub game_id: Uuid,
    #[sea_orm(unique)]
    pub ingestion_id: Uuid,
    pub map: String,
    pub started_at: DateTimeUtc,
    pub ended_at: Option<DateTimeUtc>,
    pub start_timestamp_ms: i64,
    pub end_timestamp_ms: Option<i64>,
    pub duration_ms: Option<i64>,
    pub winning_team: Option<String>,
    pub status: String,
    pub event_count: i64,
    pub player_count: i64,
    pub warnings: String,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
