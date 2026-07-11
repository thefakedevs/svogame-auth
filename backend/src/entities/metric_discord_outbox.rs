use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "metric_discord_outbox")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub game_id: Uuid,
    pub title: String,
    pub description: String,
    pub url: String,
    pub status: String,
    pub attempt_count: i32,
    pub last_error: Option<String>,
    pub next_attempt_at: DateTimeUtc,
    pub last_attempt_at: Option<DateTimeUtc>,
    pub delivered_at: Option<DateTimeUtc>,
    pub discord_channel_id: Option<String>,
    pub discord_message_id: Option<String>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
