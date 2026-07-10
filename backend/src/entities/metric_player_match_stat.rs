use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "metric_player_match_stat")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub game_id: Uuid,
    pub player_id: Uuid,
    pub kills: i64,
    pub deaths: i64,
    pub assists: i64,
    pub teamkills: i64,
    pub suicides: i64,
    pub environment_deaths: i64,
    pub damage_dealt: f64,
    pub damage_taken: f64,
    pub friendly_damage: f64,
    pub self_damage: f64,
    pub headshots: i64,
    pub headshot_damage: f64,
    pub vehicle_damage_dealt: f64,
    pub vehicle_damage_taken: f64,
    pub friendly_vehicle_damage: f64,
    pub vehicle_final_hits: i64,
    pub vehicle_destructions: i64,
    pub vehicle_teamkills: i64,
    pub vehicles_lost: i64,
    pub matches_played: i64,
    pub wins: i64,
    pub losses: i64,
    pub kills_by_weapon: String,
    pub deaths_by_source: String,
    pub damage_by_weapon: String,
    pub damage_by_source: String,
    pub vehicle_damage_by_type: String,
    pub vehicle_damage_by_weapon: String,
    pub vehicle_kills_by_type: String,
    pub vehicle_kills_by_weapon: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
