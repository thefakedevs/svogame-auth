use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, TransactionTrait,
};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::entities::{
    MetricDiscordOutboxActiveModel, MetricIngestion, MetricIngestionActiveModel,
    MetricIngestionColumn, MetricMatchActiveModel, MetricMatchPlayerActiveModel,
    MetricPlayerActiveModel, MetricPlayerColumn, MetricPlayerMatchStatActiveModel,
    MetricPlayerNicknameActiveModel, MetricPlayerNicknameColumn,
};

use super::aggregation::{MatchAggregate, PlayerAggregate, PlayerStats};
use super::parser::ParsedTimeline;

#[derive(Clone, Debug)]
pub struct PersistedIngestion {
    pub game_id: Uuid,
    pub status: &'static str,
    pub events_processed: i64,
    pub players_processed: i64,
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("a different timeline has already been processed for game {game_id}")]
    Conflict { game_id: Uuid },
    #[error("timestampMs {0} cannot be represented as UTC")]
    InvalidTimestamp(i64),
    #[error("failed to serialize metrics: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error(transparent)]
    Database(#[from] DbErr),
}

pub async fn persist_timeline(
    db: &DatabaseConnection,
    timeline: &ParsedTimeline,
    public_base_url: &str,
) -> Result<PersistedIngestion, PersistenceError> {
    if let Some(outcome) = existing_ingestion(db, timeline).await? {
        return outcome;
    }

    let now = Utc::now();
    let started_at = utc_from_millis(timeline.aggregate.start_timestamp_ms)?;
    let ended_at = timeline
        .aggregate
        .end_timestamp_ms
        .map(utc_from_millis)
        .transpose()?;
    let warnings_json = serde_json::to_string(&timeline.aggregate.warnings)?;
    let ingestion_id = Uuid::new_v4();
    let transaction = db.begin().await?;

    let ingestion_insert = MetricIngestionActiveModel {
        id: Set(ingestion_id),
        game_id: Set(timeline.aggregate.game_id),
        content_sha256: Set(timeline.content_sha256.clone()),
        status: Set(timeline.aggregate.status.to_string()),
        byte_count: Set(timeline.byte_count as i64),
        event_count: Set(timeline.aggregate.event_count as i64),
        warnings: Set(warnings_json.clone()),
        created_at: Set(now),
        completed_at: Set(now),
    }
    .insert(&transaction)
    .await;
    if let Err(error) = ingestion_insert {
        transaction.rollback().await?;
        if let Some(outcome) = existing_ingestion(db, timeline).await? {
            return outcome;
        }
        return Err(PersistenceError::Database(error));
    }

    MetricMatchActiveModel {
        game_id: Set(timeline.aggregate.game_id),
        ingestion_id: Set(ingestion_id),
        map: Set(timeline.aggregate.map.clone()),
        started_at: Set(started_at),
        ended_at: Set(ended_at),
        start_timestamp_ms: Set(timeline.aggregate.start_timestamp_ms),
        end_timestamp_ms: Set(timeline.aggregate.end_timestamp_ms),
        duration_ms: Set(timeline.aggregate.duration_ms()),
        winning_team: Set(timeline.aggregate.winning_team.clone()),
        status: Set(timeline.aggregate.status.to_string()),
        event_count: Set(timeline.aggregate.event_count as i64),
        player_count: Set(timeline.aggregate.players.len() as i64),
        warnings: Set(warnings_json),
        created_at: Set(now),
    }
    .insert(&transaction)
    .await?;

    upsert_players(&transaction, &timeline.aggregate, started_at).await?;
    insert_match_players(&transaction, &timeline.aggregate).await?;
    insert_match_stats(&transaction, &timeline.aggregate).await?;
    if timeline.aggregate.status == "completed" {
        let embed = build_match_embed(&timeline.aggregate, public_base_url);
        let embeds = legacy_discord_embeds(&timeline.aggregate);
        MetricDiscordOutboxActiveModel {
            game_id: Set(timeline.aggregate.game_id),
            title: Set(embed.title),
            description: Set(serde_json::to_string(&embeds)?),
            url: Set(embed.url),
            status: Set("pending".to_string()),
            attempt_count: Set(0),
            last_error: Set(None),
            next_attempt_at: Set(now),
            last_attempt_at: Set(None),
            delivered_at: Set(None),
            discord_channel_id: Set(None),
            discord_message_id: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&transaction)
        .await?;
    }
    transaction.commit().await?;

    Ok(PersistedIngestion {
        game_id: timeline.aggregate.game_id,
        status: if timeline.aggregate.status == "completed" {
            "processed"
        } else {
            "incomplete"
        },
        events_processed: timeline.aggregate.event_count as i64,
        players_processed: timeline.aggregate.players.len() as i64,
        warnings: timeline.aggregate.warnings.clone(),
    })
}

async fn existing_ingestion(
    db: &DatabaseConnection,
    timeline: &ParsedTimeline,
) -> Result<Option<Result<PersistedIngestion, PersistenceError>>, PersistenceError> {
    let Some(existing) = MetricIngestion::find()
        .filter(MetricIngestionColumn::GameId.eq(timeline.aggregate.game_id))
        .one(db)
        .await?
    else {
        return Ok(None);
    };
    if existing.content_sha256 == timeline.content_sha256 {
        Ok(Some(Ok(PersistedIngestion {
            game_id: timeline.aggregate.game_id,
            status: "duplicate",
            events_processed: existing.event_count,
            players_processed: timeline.aggregate.players.len() as i64,
            warnings: decode_warnings(&existing.warnings),
        })))
    } else {
        Ok(Some(Err(PersistenceError::Conflict {
            game_id: timeline.aggregate.game_id,
        })))
    }
}

async fn upsert_players(
    transaction: &sea_orm::DatabaseTransaction,
    aggregate: &MatchAggregate,
    seen_at: DateTime<Utc>,
) -> Result<(), PersistenceError> {
    let players = aggregate
        .players
        .iter()
        .map(|player| MetricPlayerActiveModel {
            player_id: Set(player.player_id),
            last_nickname: Set(player.nickname.clone()),
            first_seen_at: Set(seen_at),
            last_seen_at: Set(seen_at),
        })
        .collect::<Vec<_>>();
    crate::entities::MetricPlayer::insert_many(players)
        .on_conflict(
            OnConflict::column(MetricPlayerColumn::PlayerId)
                .update_columns([
                    MetricPlayerColumn::LastNickname,
                    MetricPlayerColumn::LastSeenAt,
                ])
                .to_owned(),
        )
        .exec_without_returning(transaction)
        .await?;

    let nicknames = aggregate
        .players
        .iter()
        .flat_map(|player| {
            let names = if player.nicknames.is_empty() {
                vec![player.nickname.clone()]
            } else {
                player.nicknames.clone()
            };
            names
                .into_iter()
                .map(|nickname| MetricPlayerNicknameActiveModel {
                    id: Set(Uuid::new_v4()),
                    player_id: Set(player.player_id),
                    nickname: Set(nickname),
                    first_seen_at: Set(seen_at),
                    last_seen_at: Set(seen_at),
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    crate::entities::MetricPlayerNickname::insert_many(nicknames)
        .on_conflict(
            OnConflict::columns([
                MetricPlayerNicknameColumn::PlayerId,
                MetricPlayerNicknameColumn::Nickname,
            ])
            .update_column(MetricPlayerNicknameColumn::LastSeenAt)
            .to_owned(),
        )
        .exec_without_returning(transaction)
        .await?;
    Ok(())
}

async fn insert_match_players(
    transaction: &sea_orm::DatabaseTransaction,
    aggregate: &MatchAggregate,
) -> Result<(), PersistenceError> {
    let models = aggregate
        .players
        .iter()
        .map(|player| MetricMatchPlayerActiveModel {
            id: Set(Uuid::new_v4()),
            game_id: Set(aggregate.game_id),
            player_id: Set(player.player_id),
            nickname: Set(player.nickname.clone()),
            initial_team: Set(player.initial_team.clone()),
            final_team: Set(player.final_team.clone()),
            changed_team: Set(player.changed_team),
            team_changes: Set(player.team_changes),
            left_count: Set(player.left_count),
            time_in_game_ms: Set(player.time_in_game_ms),
        })
        .collect::<Vec<_>>();
    crate::entities::MetricMatchPlayer::insert_many(models)
        .exec_without_returning(transaction)
        .await?;
    Ok(())
}

async fn insert_match_stats(
    transaction: &sea_orm::DatabaseTransaction,
    aggregate: &MatchAggregate,
) -> Result<(), PersistenceError> {
    let models = aggregate
        .players
        .iter()
        .map(|player| stat_model(aggregate.game_id, player))
        .collect::<Result<Vec<_>, _>>()?;
    crate::entities::MetricPlayerMatchStat::insert_many(models)
        .exec_without_returning(transaction)
        .await?;
    Ok(())
}

fn stat_model(
    game_id: Uuid,
    player: &PlayerAggregate,
) -> Result<MetricPlayerMatchStatActiveModel, PersistenceError> {
    let stats = &player.stats;
    Ok(MetricPlayerMatchStatActiveModel {
        id: Set(Uuid::new_v4()),
        game_id: Set(game_id),
        player_id: Set(player.player_id),
        kills: Set(stats.kills),
        deaths: Set(stats.deaths),
        assists: Set(stats.assists),
        teamkills: Set(stats.teamkills),
        suicides: Set(stats.suicides),
        environment_deaths: Set(stats.environment_deaths),
        damage_dealt: Set(stats.damage_dealt),
        damage_taken: Set(stats.damage_taken),
        friendly_damage: Set(stats.friendly_damage),
        self_damage: Set(stats.self_damage),
        headshots: Set(stats.headshots),
        headshot_damage: Set(stats.headshot_damage),
        vehicle_damage_dealt: Set(stats.vehicle_damage_dealt),
        vehicle_damage_taken: Set(stats.vehicle_damage_taken),
        friendly_vehicle_damage: Set(stats.friendly_vehicle_damage),
        vehicle_final_hits: Set(stats.vehicle_final_hits),
        vehicle_destructions: Set(stats.vehicle_destructions),
        vehicle_teamkills: Set(stats.vehicle_teamkills),
        vehicles_lost: Set(stats.vehicles_lost),
        matches_played: Set(stats.matches_played),
        wins: Set(stats.wins),
        losses: Set(stats.losses),
        kills_by_weapon: Set(serde_json::to_string(&stats.kills_by_weapon)?),
        deaths_by_source: Set(serde_json::to_string(&stats.deaths_by_source)?),
        damage_by_weapon: Set(serde_json::to_string(&stats.damage_by_weapon)?),
        damage_by_source: Set(serde_json::to_string(&stats.damage_by_source)?),
        vehicle_damage_by_type: Set(serde_json::to_string(&stats.vehicle_damage_by_type)?),
        vehicle_damage_by_weapon: Set(serde_json::to_string(&stats.vehicle_damage_by_weapon)?),
        vehicle_kills_by_type: Set(serde_json::to_string(&stats.vehicle_kills_by_type)?),
        vehicle_kills_by_weapon: Set(serde_json::to_string(&stats.vehicle_kills_by_weapon)?),
    })
}

fn utc_from_millis(timestamp_ms: i64) -> Result<DateTime<Utc>, PersistenceError> {
    DateTime::from_timestamp_millis(timestamp_ms)
        .ok_or(PersistenceError::InvalidTimestamp(timestamp_ms))
}

fn decode_warnings(value: &str) -> Vec<String> {
    serde_json::from_str(value).unwrap_or_else(|_| Vec::new())
}

struct MatchEmbed {
    title: String,
    #[allow(dead_code)]
    description: String,
    url: String,
}

#[allow(dead_code)]
fn legacy_discord_description(aggregate: &MatchAggregate) -> String {
    let mut lines = Vec::new();
    for team in ["Attack", "Defense", "unknown"] {
        let mut players = aggregate
            .players
            .iter()
            .filter(|player| player.final_team.as_deref().unwrap_or("unknown") == team)
            .collect::<Vec<_>>();
        if players.is_empty() {
            continue;
        }
        players.sort_by(|a, b| {
            b.stats
                .damage_dealt
                .partial_cmp(&a.stats.damage_dealt)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let team_name = match team {
            "Attack" => "🔴 Атака",
            "Defense" => "🔵 Защита",
            _ => "⚪ Неизвестно",
        };
        let team_name = if aggregate.winning_team.as_deref() == Some(team) {
            format!("🏆 {team_name}")
        } else {
            team_name.to_string()
        };
        lines.push(format!("**{team_name}**"));
        lines.extend(players.iter().enumerate().map(|(i, p)| {
            format!(
                "{}. **{}** → {}/{}/{} • {:.0} DMG",
                i + 1,
                truncate_chars(&p.nickname, 32),
                p.stats.kills,
                p.stats.deaths,
                p.stats.assists,
                p.stats.damage_dealt
            )
        }));
    }
    let special = [
        (
            "🔫 **Больше всего тимкиллов:**",
            aggregate.players.iter().max_by_key(|p| p.stats.teamkills),
        ),
        (
            "💥 **Уничтожено техники:**",
            aggregate
                .players
                .iter()
                .max_by_key(|p| p.stats.vehicle_destructions),
        ),
        (
            "💀 **Больше всего самоубийств:**",
            aggregate.players.iter().max_by_key(|p| p.stats.suicides),
        ),
    ];
    let highlights = special
        .into_iter()
        .filter_map(|(label, player)| {
            player.and_then(|p| {
                let count = if label.contains("тимкиллов") {
                    p.stats.teamkills
                } else if label.contains("техники") {
                    p.stats.vehicle_destructions
                } else {
                    p.stats.suicides
                };
                (count > 0).then(|| format!("{label} {} ({count})", p.nickname))
            })
        })
        .collect::<Vec<_>>();
    if !highlights.is_empty() {
        lines.push("📊 **Особые достижения**".to_string());
        lines.extend(highlights);
    }
    truncate_chars(&lines.join("\n\n"), 4000)
}

fn legacy_discord_embeds(aggregate: &MatchAggregate) -> Vec<Value> {
    const EMBED_CHAR_LIMIT: usize = 4096;
    const FIELD_VALUE_LIMIT: usize = 1024;
    let duration_seconds = aggregate.duration_ms().unwrap_or_default() / 1000;
    let footer = json!({ "text": format!("⏱ Длительность: {:02}:{:02}", duration_seconds / 60, duration_seconds % 60) });
    let mut fields = Vec::new();
    for team in ["Attack", "Defense"] {
        let mut players = aggregate
            .players
            .iter()
            .filter(|player| player.final_team.as_deref() == Some(team))
            .collect::<Vec<_>>();
        if players.is_empty() {
            continue;
        }
        players.sort_by(|left, right| {
            right
                .stats
                .damage_dealt
                .partial_cmp(&left.stats.damage_dealt)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let base_name = if team == "Attack" {
            "🔴 Атака"
        } else {
            "🔵 Защита"
        };
        let name = if aggregate.winning_team.as_deref() == Some(team) {
            format!("🏆 {base_name}")
        } else {
            base_name.to_string()
        };
        let value = players
            .iter()
            .enumerate()
            .map(|(index, player)| {
                format!(
                    "{}. **{}** → {}K/{}D/{}A • {:.0} DMG",
                    index + 1,
                    truncate_chars(&player.nickname, 128),
                    player.stats.kills,
                    player.stats.deaths,
                    player.stats.assists,
                    player.stats.damage_dealt
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        fields.push(json!({ "name": name, "value": truncate_chars(&value, FIELD_VALUE_LIMIT), "inline": false }));
    }
    let highlights = legacy_highlights(&aggregate.players);
    if !highlights.is_empty() {
        fields.push(json!({
            "name": "📊 Особые достижения",
            "value": truncate_chars(&highlights.join("\n"), FIELD_VALUE_LIMIT),
            "inline": false
        }));
    }
    let mut embeds = Vec::new();
    let mut current_fields = Vec::new();
    let mut current_size = "🎮 Статистика игры".chars().count();
    for field in fields {
        let field_size = field["name"].as_str().unwrap_or_default().chars().count()
            + field["value"].as_str().unwrap_or_default().chars().count();
        if !current_fields.is_empty() && current_size + field_size > EMBED_CHAR_LIMIT {
            embeds.push(json!({ "title": "🎮 Статистика игры", "color": 3447003, "footer": footer, "fields": current_fields }));
            current_fields = Vec::new();
            current_size = "🎮 Статистика игры (продолжение)".chars().count();
        }
        current_size += field_size;
        current_fields.push(field);
    }
    if !current_fields.is_empty() {
        let title = if embeds.is_empty() {
            "🎮 Статистика игры"
        } else {
            "🎮 Статистика игры (продолжение)"
        };
        embeds.push(
            json!({ "title": title, "color": 3447003, "footer": footer, "fields": current_fields }),
        );
    }
    embeds.push(json!({
        "color": 0x005ed5,
        "author": {
            "name": "Спонсор сервера: lifehosting.pro",
            "url": "https://discord.lifehosting.pro",
            "icon_url": "https://cdn.discordapp.com/icons/944864952764735488/66efbabc194f39d9d6db36e6628e55bf.png?size=512"
        }
    }));
    embeds
}

fn legacy_highlights(players: &[PlayerAggregate]) -> Vec<String> {
    let mut highlights = Vec::new();
    if let Some(player) = players.iter().max_by_key(|player| player.stats.teamkills)
        && player.stats.teamkills > 0
    {
        highlights.push(format!(
            "🔫 **Больше всего тимкиллов:** {} ({} тимкиллов)",
            player.nickname, player.stats.teamkills
        ));
    }
    if let Some(player) = players
        .iter()
        .max_by_key(|player| player.stats.vehicle_destructions)
        && player.stats.vehicle_destructions > 0
    {
        highlights.push(format!(
            "💥 **Уничтожено техники:** {} ({} уничтожено)",
            player.nickname, player.stats.vehicle_destructions
        ));
    }
    if let Some(player) = players.iter().max_by_key(|player| player.stats.suicides)
        && player.stats.suicides > 0
    {
        highlights.push(format!(
            "💀 **Больше всего самоубийств:** {} ({} самоубийств)",
            player.nickname, player.stats.suicides
        ));
    }
    highlights
}

fn build_match_embed(aggregate: &MatchAggregate, _public_base_url: &str) -> MatchEmbed {
    let totals = aggregate
        .players
        .iter()
        .fold(PlayerStats::default(), |mut acc, player| {
            acc.kills += player.stats.kills;
            acc.deaths += player.stats.deaths;
            acc.assists += player.stats.assists;
            acc.vehicle_destructions += player.stats.vehicle_destructions;
            acc
        });
    let duration = aggregate.duration_ms().unwrap_or_default() / 1000;
    let mut description = format!(
        "**Карта:** {}\n**Матч:** `{}`\n**Длительность:** {:02}:{:02}\n**Победитель:** {}\n**Игроков:** {}\n**Итого:** {}K / {}D / {}A / {} техники",
        truncate_chars(&aggregate.map, 100),
        aggregate.game_id,
        duration / 60,
        duration % 60,
        aggregate.winning_team.as_deref().unwrap_or("не определён"),
        aggregate.players.len(),
        totals.kills,
        totals.deaths,
        totals.assists,
        totals.vehicle_destructions,
    );
    for (label, selector) in [
        ("Топ по убийствам", MetricSelector::Kills),
        ("Топ по помощям", MetricSelector::Assists),
        ("Топ по технике", MetricSelector::Vehicles),
    ] {
        let rows = top_players(&aggregate.players, selector);
        if !rows.is_empty() {
            description.push_str(&format!("\n\n**{label}:**\n{}", rows.join("\n")));
        }
    }
    MatchEmbed {
        title: "Матч завершён".to_string(),
        description: truncate_chars(&description, 4000),
        url: String::new(),
    }
}

#[derive(Copy, Clone)]
enum MetricSelector {
    Kills,
    Assists,
    Vehicles,
}

fn top_players(players: &[PlayerAggregate], selector: MetricSelector) -> Vec<String> {
    let mut players = players
        .iter()
        .map(|player| {
            let value = match selector {
                MetricSelector::Kills => player.stats.kills,
                MetricSelector::Assists => player.stats.assists,
                MetricSelector::Vehicles => player.stats.vehicle_destructions,
            };
            (player, value)
        })
        .filter(|(_, value)| *value > 0)
        .collect::<Vec<_>>();
    players.sort_by(|(left, left_value), (right, right_value)| {
        right_value
            .cmp(left_value)
            .then_with(|| left.nickname.cmp(&right.nickname))
            .then_with(|| left.player_id.cmp(&right.player_id))
    });
    players
        .into_iter()
        .take(3)
        .enumerate()
        .map(|(index, (player, value))| {
            format!(
                "{}. {} — {}",
                index + 1,
                truncate_chars(&player.nickname, 32),
                value
            )
        })
        .collect()
}

fn truncate_chars(value: &str, limit: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(limit).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}…")
    } else {
        truncated
    }
}

pub fn merge_stats(target: &mut PlayerStats, source: &crate::entities::MetricPlayerMatchStatModel) {
    target.kills += source.kills;
    target.deaths += source.deaths;
    target.assists += source.assists;
    target.teamkills += source.teamkills;
    target.suicides += source.suicides;
    target.environment_deaths += source.environment_deaths;
    target.damage_dealt += source.damage_dealt;
    target.damage_taken += source.damage_taken;
    target.friendly_damage += source.friendly_damage;
    target.self_damage += source.self_damage;
    target.headshots += source.headshots;
    target.headshot_damage += source.headshot_damage;
    target.vehicle_damage_dealt += source.vehicle_damage_dealt;
    target.vehicle_damage_taken += source.vehicle_damage_taken;
    target.friendly_vehicle_damage += source.friendly_vehicle_damage;
    target.vehicle_final_hits += source.vehicle_final_hits;
    target.vehicle_destructions += source.vehicle_destructions;
    target.vehicle_teamkills += source.vehicle_teamkills;
    target.vehicles_lost += source.vehicles_lost;
    target.matches_played += source.matches_played;
    target.wins += source.wins;
    target.losses += source.losses;
    merge_i64_map(&mut target.kills_by_weapon, &source.kills_by_weapon);
    merge_i64_map(&mut target.deaths_by_source, &source.deaths_by_source);
    merge_f64_map(&mut target.damage_by_weapon, &source.damage_by_weapon);
    merge_f64_map(&mut target.damage_by_source, &source.damage_by_source);
    merge_f64_map(
        &mut target.vehicle_damage_by_type,
        &source.vehicle_damage_by_type,
    );
    merge_f64_map(
        &mut target.vehicle_damage_by_weapon,
        &source.vehicle_damage_by_weapon,
    );
    merge_i64_map(
        &mut target.vehicle_kills_by_type,
        &source.vehicle_kills_by_type,
    );
    merge_i64_map(
        &mut target.vehicle_kills_by_weapon,
        &source.vehicle_kills_by_weapon,
    );
}

fn merge_i64_map(target: &mut std::collections::BTreeMap<String, i64>, json: &str) {
    let values: HashMap<String, i64> = serde_json::from_str(json).unwrap_or_default();
    for (key, value) in values {
        *target.entry(key).or_default() += value;
    }
}

fn merge_f64_map(target: &mut std::collections::BTreeMap<String, f64>, json: &str) {
    let values: HashMap<String, f64> = serde_json::from_str(json).unwrap_or_default();
    for (key, value) in values {
        *target.entry(key).or_default() += value;
    }
}
