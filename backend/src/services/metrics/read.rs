use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use crate::entities::{
    MetricMatch, MetricMatchColumn, MetricMatchModel, MetricMatchPlayer, MetricMatchPlayerColumn,
    MetricMatchPlayerModel, MetricPlayer, MetricPlayerColumn, MetricPlayerMatchStat,
    MetricPlayerMatchStatColumn, MetricPlayerMatchStatModel, MetricPlayerModel,
    MetricPlayerNickname, MetricPlayerNicknameColumn, MetricPlayerNicknameModel, User, UserColumn,
};

use super::aggregation::PlayerStats;
use super::persistence::merge_stats;

#[derive(Clone, Debug, Default)]
pub struct TimeRange {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug)]
pub struct MatchBundle {
    pub match_model: MetricMatchModel,
    pub players: Vec<(MetricMatchPlayerModel, MetricPlayerMatchStatModel)>,
}

#[derive(Clone, Debug)]
pub struct PlayerMatch {
    pub match_model: MetricMatchModel,
    pub final_team: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PlayerProfileBundle {
    pub player: MetricPlayerModel,
    pub nicknames: Vec<MetricPlayerNicknameModel>,
}

#[derive(Clone, Debug, Default)]
pub struct PlayerSummary {
    pub stats: PlayerStats,
    pub time_in_game_ms: i64,
}

#[derive(Clone, Debug)]
pub struct LeaderboardRow {
    pub player_id: Uuid,
    pub nickname: String,
    pub summary: PlayerSummary,
    pub metric_value: f64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LeaderboardMetric {
    Kills,
    Deaths,
    Assists,
    VehicleDestructions,
    Kd,
    Kda,
    DamageDealt,
    DamagePerMinute,
    WinRate,
    Headshots,
    TimeInGame,
}

impl LeaderboardMetric {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "kills" => Some(Self::Kills),
            "deaths" => Some(Self::Deaths),
            "assists" => Some(Self::Assists),
            "vehicle_destructions" => Some(Self::VehicleDestructions),
            "kd" => Some(Self::Kd),
            "kda" => Some(Self::Kda),
            "damage_dealt" => Some(Self::DamageDealt),
            "damage_per_minute" => Some(Self::DamagePerMinute),
            "win_rate" => Some(Self::WinRate),
            "headshots" => Some(Self::Headshots),
            "time_in_game" => Some(Self::TimeInGame),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Kills => "kills",
            Self::Deaths => "deaths",
            Self::Assists => "assists",
            Self::VehicleDestructions => "vehicle_destructions",
            Self::Kd => "kd",
            Self::Kda => "kda",
            Self::DamageDealt => "damage_dealt",
            Self::DamagePerMinute => "damage_per_minute",
            Self::WinRate => "win_rate",
            Self::Headshots => "headshots",
            Self::TimeInGame => "time_in_game",
        }
    }

    fn value(self, summary: &PlayerSummary) -> f64 {
        let stats = &summary.stats;
        match self {
            Self::Kills => stats.kills as f64,
            Self::Deaths => stats.deaths as f64,
            Self::Assists => stats.assists as f64,
            Self::VehicleDestructions => stats.vehicle_destructions as f64,
            Self::Kd => safe_ratio(stats.kills, stats.deaths),
            Self::Kda => safe_ratio(stats.kills + stats.assists, stats.deaths),
            Self::DamageDealt => stats.damage_dealt,
            Self::DamagePerMinute => {
                let minutes = summary.time_in_game_ms as f64 / 60_000.0;
                if minutes > 0.0 {
                    stats.damage_dealt / minutes
                } else {
                    0.0
                }
            }
            Self::WinRate => {
                if stats.matches_played > 0 {
                    stats.wins as f64 * 100.0 / stats.matches_played as f64
                } else {
                    0.0
                }
            }
            Self::Headshots => stats.headshots as f64,
            Self::TimeInGame => summary.time_in_game_ms as f64,
        }
    }
}

pub async fn get_match(
    db: &DatabaseConnection,
    game_id: Uuid,
) -> Result<Option<MatchBundle>, DbErr> {
    let Some(match_model) = MetricMatch::find_by_id(game_id).one(db).await? else {
        return Ok(None);
    };
    let participation = MetricMatchPlayer::find()
        .filter(MetricMatchPlayerColumn::GameId.eq(game_id))
        .all(db)
        .await?;
    let stats = MetricPlayerMatchStat::find()
        .filter(MetricPlayerMatchStatColumn::GameId.eq(game_id))
        .all(db)
        .await?;
    let mut stats_by_player = stats
        .into_iter()
        .map(|stats| (stats.player_id, stats))
        .collect::<HashMap<_, _>>();
    let mut players = participation
        .into_iter()
        .filter_map(|player| {
            stats_by_player
                .remove(&player.player_id)
                .map(|stats| (player, stats))
        })
        .collect::<Vec<_>>();
    players.sort_by(|(left, _), (right, _)| {
        left.final_team
            .cmp(&right.final_team)
            .then_with(|| left.nickname.cmp(&right.nickname))
            .then_with(|| left.player_id.cmp(&right.player_id))
    });
    Ok(Some(MatchBundle {
        match_model,
        players,
    }))
}

pub async fn get_player_profile(
    db: &DatabaseConnection,
    player_id: Uuid,
) -> Result<Option<PlayerProfileBundle>, DbErr> {
    let Some(player) = MetricPlayer::find_by_id(player_id).one(db).await? else {
        return Ok(None);
    };
    let nicknames = MetricPlayerNickname::find()
        .filter(MetricPlayerNicknameColumn::PlayerId.eq(player_id))
        .order_by_asc(MetricPlayerNicknameColumn::FirstSeenAt)
        .all(db)
        .await?;
    Ok(Some(PlayerProfileBundle { player, nicknames }))
}

pub async fn get_player_matches(
    db: &DatabaseConnection,
    player_id: Uuid,
    range: &TimeRange,
) -> Result<Vec<PlayerMatch>, DbErr> {
    let participation = MetricMatchPlayer::find()
        .filter(MetricMatchPlayerColumn::PlayerId.eq(player_id))
        .all(db)
        .await?;
    let teams_by_game = participation
        .iter()
        .map(|model| (model.game_id, model.final_team.clone()))
        .collect::<HashMap<_, _>>();
    let game_ids = teams_by_game.keys().copied().collect::<Vec<_>>();
    if game_ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut query = MetricMatch::find().filter(MetricMatchColumn::GameId.is_in(game_ids));
    query = apply_range(query, range);
    let matches = query
        .order_by_desc(MetricMatchColumn::StartedAt)
        .all(db)
        .await?;
    Ok(matches
        .into_iter()
        .map(|match_model| PlayerMatch {
            final_team: teams_by_game.get(&match_model.game_id).cloned().flatten(),
            match_model,
        })
        .collect())
}

pub async fn get_player_summary(
    db: &DatabaseConnection,
    player_id: Uuid,
    range: &TimeRange,
) -> Result<PlayerSummary, DbErr> {
    let game_ids = complete_game_ids(db, range).await?;
    if game_ids.is_empty() {
        return Ok(PlayerSummary::default());
    }
    let stats = MetricPlayerMatchStat::find()
        .filter(MetricPlayerMatchStatColumn::PlayerId.eq(player_id))
        .filter(MetricPlayerMatchStatColumn::GameId.is_in(game_ids.clone()))
        .all(db)
        .await?;
    let time_in_game_ms = MetricMatchPlayer::find()
        .filter(MetricMatchPlayerColumn::PlayerId.eq(player_id))
        .filter(MetricMatchPlayerColumn::GameId.is_in(game_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|model| model.time_in_game_ms)
        .sum();
    let mut summary = PlayerSummary {
        stats: PlayerStats::default(),
        time_in_game_ms,
    };
    for stats in stats {
        merge_stats(&mut summary.stats, &stats);
    }
    Ok(summary)
}

pub async fn leaderboard(
    db: &DatabaseConnection,
    range: &TimeRange,
    metric: LeaderboardMetric,
    min_matches: i64,
    descending: bool,
) -> Result<Vec<LeaderboardRow>, DbErr> {
    let game_ids = complete_game_ids(db, range).await?;
    if game_ids.is_empty() {
        return Ok(Vec::new());
    }
    let stats = MetricPlayerMatchStat::find()
        .filter(MetricPlayerMatchStatColumn::GameId.is_in(game_ids.clone()))
        .all(db)
        .await?;
    let participation = MetricMatchPlayer::find()
        .filter(MetricMatchPlayerColumn::GameId.is_in(game_ids))
        .all(db)
        .await?;
    let mut summaries: HashMap<Uuid, PlayerSummary> = HashMap::new();
    for stats in stats {
        merge_stats(
            &mut summaries.entry(stats.player_id).or_default().stats,
            &stats,
        );
    }
    for player in participation {
        summaries
            .entry(player.player_id)
            .or_default()
            .time_in_game_ms += player.time_in_game_ms;
    }
    summaries.retain(|_, summary| summary.stats.matches_played >= min_matches);
    let player_ids = summaries.keys().copied().collect::<HashSet<_>>();
    if player_ids.is_empty() {
        return Ok(Vec::new());
    }
    let nicknames = MetricPlayer::find()
        .filter(MetricPlayerColumn::PlayerId.is_in(player_ids.iter().copied()))
        .all(db)
        .await?
        .into_iter()
        .map(|player| (player.player_id, player.last_nickname))
        .collect::<HashMap<_, _>>();
    let backend_nicknames = User::find()
        .filter(UserColumn::Id.is_in(player_ids.iter().copied()))
        .all(db)
        .await?
        .into_iter()
        .map(|user| (user.id, user.username))
        .collect::<HashMap<_, _>>();
    let mut rows = summaries
        .into_iter()
        .map(|(player_id, summary)| LeaderboardRow {
            player_id,
            nickname: backend_nicknames
                .get(&player_id)
                .cloned()
                .or_else(|| nicknames.get(&player_id).cloned())
                .unwrap_or_else(|| player_id.to_string()),
            metric_value: metric.value(&summary),
            summary,
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        let metric_order = if descending {
            right.metric_value.total_cmp(&left.metric_value)
        } else {
            left.metric_value.total_cmp(&right.metric_value)
        };
        metric_order
            .then_with(|| right.summary.stats.kills.cmp(&left.summary.stats.kills))
            .then_with(|| left.summary.stats.deaths.cmp(&right.summary.stats.deaths))
            .then_with(|| left.player_id.cmp(&right.player_id))
    });
    Ok(rows)
}

async fn complete_game_ids(db: &DatabaseConnection, range: &TimeRange) -> Result<Vec<Uuid>, DbErr> {
    let query = MetricMatch::find().filter(MetricMatchColumn::Status.eq("completed"));
    apply_range(query, range)
        .all(db)
        .await
        .map(|matches| matches.into_iter().map(|model| model.game_id).collect())
}

fn apply_range(
    mut query: sea_orm::Select<crate::entities::metric_match::Entity>,
    range: &TimeRange,
) -> sea_orm::Select<crate::entities::metric_match::Entity> {
    if let Some(from) = range.from {
        query = query.filter(MetricMatchColumn::StartedAt.gte(from));
    }
    if let Some(to) = range.to {
        query = query.filter(MetricMatchColumn::StartedAt.lte(to));
    }
    query
}

fn safe_ratio(numerator: i64, denominator: i64) -> f64 {
    numerator as f64 / denominator.max(1) as f64
}
