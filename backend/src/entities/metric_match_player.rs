use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "metric_match_player")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub game_id: Uuid,
    pub player_id: Uuid,
    pub nickname: String,
    pub initial_team: Option<String>,
    pub final_team: Option<String>,
    pub changed_team: bool,
    pub team_changes: i64,
    pub left_count: i64,
    pub time_in_game_ms: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
