use std::collections::{BTreeMap, HashMap, HashSet};

use uuid::Uuid;

use super::model::{DamageSource, TimelineEvent};

const ASSIST_THRESHOLD: f64 = 0.30;

#[derive(Clone, Debug, Default, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlayerStats {
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
    pub kills_by_weapon: BTreeMap<String, i64>,
    pub deaths_by_source: BTreeMap<String, i64>,
    pub damage_by_weapon: BTreeMap<String, f64>,
    pub damage_by_source: BTreeMap<String, f64>,
    pub vehicle_damage_by_type: BTreeMap<String, f64>,
    pub vehicle_damage_by_weapon: BTreeMap<String, f64>,
    pub vehicle_kills_by_type: BTreeMap<String, i64>,
    pub vehicle_kills_by_weapon: BTreeMap<String, i64>,
}

#[derive(Clone, Debug)]
pub struct PlayerAggregate {
    pub player_id: Uuid,
    pub nickname: String,
    pub nicknames: Vec<String>,
    pub initial_team: Option<String>,
    pub final_team: Option<String>,
    pub changed_team: bool,
    pub team_changes: i64,
    pub left_count: i64,
    pub time_in_game_ms: i64,
    pub stats: PlayerStats,
}

#[derive(Clone, Debug)]
pub struct MatchAggregate {
    pub game_id: Uuid,
    pub map: String,
    pub start_timestamp_ms: i64,
    pub end_timestamp_ms: Option<i64>,
    pub winning_team: Option<String>,
    pub status: &'static str,
    pub event_count: u64,
    pub event_counts: BTreeMap<String, u64>,
    pub warnings: Vec<String>,
    pub players: Vec<PlayerAggregate>,
}

impl MatchAggregate {
    pub fn duration_ms(&self) -> Option<i64> {
        self.end_timestamp_ms
            .map(|end| end.saturating_sub(self.start_timestamp_ms))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AggregationError {
    #[error("timeline does not contain game_started")]
    MissingGameStarted,
    #[error("timeline contains more than one game_started")]
    DuplicateGameStarted,
    #[error("timeline contains more than one game_ended")]
    DuplicateGameEnded,
    #[error("event gameId {actual} does not match URL gameId {expected}")]
    GameIdMismatch { expected: Uuid, actual: Uuid },
    #[error("game_ended timestamp precedes game_started")]
    EndBeforeStart,
    #[error("timeline does not contain any players")]
    NoPlayers,
}

#[derive(Default)]
struct PlayerState {
    nickname: Option<String>,
    nicknames: HashSet<String>,
    initial_team: Option<String>,
    current_team: Option<String>,
    active_since: Option<i64>,
    changed_team: bool,
    team_changes: i64,
    left_count: i64,
    time_in_game_ms: i64,
    stats: PlayerStats,
}

pub struct TimelineAggregator {
    expected_game_id: Uuid,
    map: Option<String>,
    start_timestamp_ms: Option<i64>,
    end_timestamp_ms: Option<i64>,
    winning_team: Option<String>,
    players: HashMap<Uuid, PlayerState>,
    damage_context: HashMap<Uuid, HashMap<Uuid, f64>>,
    fatal_vehicle_entities: HashSet<String>,
    destroyed_vehicle_entities: HashSet<String>,
    event_count: u64,
    event_counts: BTreeMap<String, u64>,
    previous_timestamp_ms: Option<i64>,
    warnings: Vec<String>,
    unknown_player_warnings: HashSet<Uuid>,
}

impl TimelineAggregator {
    pub fn new(expected_game_id: Uuid) -> Self {
        Self {
            expected_game_id,
            map: None,
            start_timestamp_ms: None,
            end_timestamp_ms: None,
            winning_team: None,
            players: HashMap::new(),
            damage_context: HashMap::new(),
            fatal_vehicle_entities: HashSet::new(),
            destroyed_vehicle_entities: HashSet::new(),
            event_count: 0,
            event_counts: BTreeMap::new(),
            previous_timestamp_ms: None,
            warnings: Vec::new(),
            unknown_player_warnings: HashSet::new(),
        }
    }

    pub fn apply(&mut self, event: TimelineEvent, line: u64) -> Result<(), AggregationError> {
        self.event_count += 1;
        *self
            .event_counts
            .entry(event.event_type().to_string())
            .or_default() += 1;
        if let Some(timestamp_ms) = event.timestamp_ms() {
            if let Some(previous) = self.previous_timestamp_ms
                && timestamp_ms < previous
            {
                self.warn(format!(
                    "line {line}: timestampMs moved backwards from {previous} to {timestamp_ms}; source order was preserved"
                ));
            }
            self.previous_timestamp_ms = Some(timestamp_ms);
            if let Some(end) = self.end_timestamp_ms
                && timestamp_ms > end
            {
                self.warn(format!(
                    "line {line}: event {} occurs after game_ended",
                    event.event_type()
                ));
            }
        }

        match event {
            TimelineEvent::GameStarted {
                timestamp_ms,
                game_id,
                map,
            } => {
                self.validate_game_id(game_id)?;
                if self.start_timestamp_ms.is_some() {
                    return Err(AggregationError::DuplicateGameStarted);
                }
                self.start_timestamp_ms = Some(timestamp_ms);
                self.map = Some(map);
            }
            TimelineEvent::GameEnded {
                timestamp_ms,
                game_id,
                winning_team,
            } => {
                self.validate_game_id(game_id)?;
                if self.end_timestamp_ms.is_some() {
                    return Err(AggregationError::DuplicateGameEnded);
                }
                self.end_timestamp_ms = Some(timestamp_ms);
                self.winning_team = winning_team;
            }
            TimelineEvent::PlayerJoined {
                timestamp_ms,
                player_id,
                nickname,
                team,
            } => self.player_joined(player_id, nickname, team, timestamp_ms, line),
            TimelineEvent::PlayerLeft {
                timestamp_ms,
                player_id,
            } => self.player_left(player_id, timestamp_ms, line),
            TimelineEvent::PlayerTeamChanged {
                player_id,
                new_team,
                ..
            } => self.player_team_changed(player_id, new_team, line),
            TimelineEvent::PlayerDamage {
                victim_id,
                attacker_id,
                damage,
                source,
                headshot,
                ..
            } => self.player_damage(victim_id, attacker_id, damage, &source, headshot, line),
            TimelineEvent::PlayerDeath {
                victim_id,
                killer_id,
                source,
                ..
            } => self.player_death(victim_id, killer_id, &source, line),
            TimelineEvent::VehicleDamage {
                vehicle_entity_id,
                vehicle_type,
                attacker_id,
                vehicle_owner_id,
                damage,
                source,
                is_fatal,
                ..
            } => self.vehicle_damage(
                vehicle_entity_id,
                vehicle_type,
                attacker_id,
                vehicle_owner_id,
                damage,
                &source,
                is_fatal,
                line,
            ),
            TimelineEvent::VehicleDestroyed {
                vehicle_entity_id,
                vehicle_type,
                attacker_id,
                vehicle_owner_id,
                source,
                is_team_kill,
                ..
            } => self.vehicle_destroyed(
                vehicle_entity_id,
                vehicle_type,
                attacker_id,
                vehicle_owner_id,
                &source,
                is_team_kill,
                line,
            ),
            TimelineEvent::Ignored {
                event_type, known, ..
            } => {
                if !known {
                    self.warn(format!(
                        "line {line}: unknown event type `{event_type}` ignored"
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn finish(mut self) -> Result<MatchAggregate, AggregationError> {
        let start = self
            .start_timestamp_ms
            .ok_or(AggregationError::MissingGameStarted)?;
        if let Some(end) = self.end_timestamp_ms
            && end < start
        {
            return Err(AggregationError::EndBeforeStart);
        }
        if self.players.is_empty() {
            return Err(AggregationError::NoPlayers);
        }
        if let Some(end) = self.end_timestamp_ms {
            for player in self.players.values_mut() {
                close_presence(player, end, &mut self.warnings);
            }
        } else {
            self.warn(
                "game_ended is missing; active presence intervals remain open and the match is excluded from leaderboards"
                    .to_string(),
            );
        }

        let completed = self.end_timestamp_ms.is_some();
        let winning_team = self.winning_team.clone();
        let mut players = self
            .players
            .into_iter()
            .map(|(player_id, mut player)| {
                player.stats.matches_played = i64::from(completed);
                if completed
                    && let (Some(winner), Some(team)) = (
                        winning_team.as_deref(),
                        valid_team(player.current_team.as_deref()),
                    )
                {
                    if winner == team {
                        player.stats.wins = 1;
                    } else {
                        player.stats.losses = 1;
                    }
                }
                let mut nicknames: Vec<_> = player.nicknames.into_iter().collect();
                nicknames.sort();
                PlayerAggregate {
                    player_id,
                    nickname: player
                        .nickname
                        .unwrap_or_else(|| fallback_nickname(player_id)),
                    nicknames,
                    initial_team: player.initial_team,
                    final_team: player.current_team,
                    changed_team: player.changed_team,
                    team_changes: player.team_changes,
                    left_count: player.left_count,
                    time_in_game_ms: player.time_in_game_ms,
                    stats: player.stats,
                }
            })
            .collect::<Vec<_>>();
        players.sort_by_key(|player| player.player_id);

        Ok(MatchAggregate {
            game_id: self.expected_game_id,
            map: self.map.unwrap_or_else(|| "unknown".to_string()),
            start_timestamp_ms: start,
            end_timestamp_ms: self.end_timestamp_ms,
            winning_team: self.winning_team,
            status: if completed { "completed" } else { "incomplete" },
            event_count: self.event_count,
            event_counts: self.event_counts,
            warnings: self.warnings,
            players,
        })
    }

    fn validate_game_id(&self, actual: Uuid) -> Result<(), AggregationError> {
        if actual == self.expected_game_id {
            Ok(())
        } else {
            Err(AggregationError::GameIdMismatch {
                expected: self.expected_game_id,
                actual,
            })
        }
    }

    fn player_joined(
        &mut self,
        player_id: Uuid,
        nickname: String,
        team: Option<String>,
        timestamp_ms: i64,
        line: u64,
    ) {
        let player = self.players.entry(player_id).or_default();
        if player.active_since.is_some() {
            self.warn(format!(
                "line {line}: duplicate player_joined for {player_id}; existing interval was kept"
            ));
            return;
        }
        player.nicknames.insert(nickname.clone());
        player.nickname = Some(nickname);
        if player.initial_team.is_none() {
            player.initial_team = team.clone();
        }
        player.current_team = team;
        player.active_since = Some(timestamp_ms);
    }

    fn player_left(&mut self, player_id: Uuid, timestamp_ms: i64, line: u64) {
        self.ensure_player(player_id, line);
        let Some(player) = self.players.get_mut(&player_id) else {
            return;
        };
        player.left_count += 1;
        if let Some(joined_at) = player.active_since.take() {
            if timestamp_ms >= joined_at {
                player.time_in_game_ms = player
                    .time_in_game_ms
                    .saturating_add(timestamp_ms - joined_at);
            } else {
                self.warn(format!(
                    "line {line}: player_left for {player_id} precedes its player_joined"
                ));
            }
        } else {
            self.warn(format!(
                "line {line}: player_left for {player_id} has no open presence interval"
            ));
        }
    }

    fn player_team_changed(&mut self, player_id: Uuid, new_team: String, line: u64) {
        self.ensure_player(player_id, line);
        if let Some(player) = self.players.get_mut(&player_id) {
            player.current_team = Some(new_team);
            player.team_changes += 1;
            player.changed_team = true;
        }
        self.damage_context.remove(&player_id);
    }

    fn player_damage(
        &mut self,
        victim_id: Uuid,
        attacker_id: Option<Uuid>,
        damage: f64,
        source: &DamageSource,
        headshot: bool,
        line: u64,
    ) {
        self.ensure_player(victim_id, line);
        if let Some(victim) = self.players.get_mut(&victim_id) {
            victim.stats.damage_taken += damage;
        }
        let Some(attacker_id) = attacker_id else {
            return;
        };
        self.ensure_player(attacker_id, line);
        let friendly = attacker_id != victim_id && self.same_team(attacker_id, victim_id);
        if let Some(attacker) = self.players.get_mut(&attacker_id) {
            attacker.stats.damage_dealt += damage;
            add_f64(
                &mut attacker.stats.damage_by_weapon,
                source.breakdown_key(),
                damage,
            );
            add_f64(
                &mut attacker.stats.damage_by_source,
                source.kind.clone(),
                damage,
            );
            if attacker_id == victim_id {
                attacker.stats.self_damage += damage;
            } else if friendly {
                attacker.stats.friendly_damage += damage;
            }
            if headshot {
                attacker.stats.headshots += 1;
                attacker.stats.headshot_damage += damage;
            }
        }
        if damage > 0.0 {
            *self
                .damage_context
                .entry(victim_id)
                .or_default()
                .entry(attacker_id)
                .or_default() += damage;
        }
    }

    fn player_death(
        &mut self,
        victim_id: Uuid,
        killer_id: Option<Uuid>,
        source: &DamageSource,
        line: u64,
    ) {
        self.ensure_player(victim_id, line);
        if let Some(victim) = self.players.get_mut(&victim_id) {
            victim.stats.deaths += 1;
            *victim
                .stats
                .deaths_by_source
                .entry(source.kind.clone())
                .or_default() += 1;
        }
        match killer_id {
            None => {
                if let Some(victim) = self.players.get_mut(&victim_id) {
                    victim.stats.suicides += 1;
                    victim.stats.environment_deaths += 1;
                }
            }
            Some(killer_id) if killer_id == victim_id => {
                if let Some(victim) = self.players.get_mut(&victim_id) {
                    victim.stats.suicides += 1;
                }
            }
            Some(killer_id) => {
                self.ensure_player(killer_id, line);
                if self.same_team(killer_id, victim_id) {
                    if let Some(killer) = self.players.get_mut(&killer_id) {
                        killer.stats.teamkills += 1;
                    }
                } else {
                    if let Some(killer) = self.players.get_mut(&killer_id) {
                        killer.stats.kills += 1;
                        *killer
                            .stats
                            .kills_by_weapon
                            .entry(source.breakdown_key())
                            .or_default() += 1;
                    }
                    self.award_assists(victim_id, killer_id);
                }
            }
        }
        self.damage_context.remove(&victim_id);
    }

    #[allow(clippy::too_many_arguments)]
    fn vehicle_damage(
        &mut self,
        vehicle_entity_id: String,
        vehicle_type: Option<String>,
        attacker_id: Option<Uuid>,
        owner_id: Option<Uuid>,
        damage: f64,
        source: &DamageSource,
        is_fatal: bool,
        line: u64,
    ) {
        if let Some(owner_id) = owner_id {
            self.ensure_player(owner_id, line);
            if let Some(owner) = self.players.get_mut(&owner_id) {
                owner.stats.vehicle_damage_taken += damage;
            }
        }
        let Some(attacker_id) = attacker_id else {
            return;
        };
        self.ensure_player(attacker_id, line);
        let friendly = owner_id
            .filter(|owner_id| *owner_id != attacker_id)
            .is_some_and(|owner_id| self.same_team(attacker_id, owner_id));
        if let Some(attacker) = self.players.get_mut(&attacker_id) {
            attacker.stats.vehicle_damage_dealt += damage;
            if friendly {
                attacker.stats.friendly_vehicle_damage += damage;
            }
            add_f64(
                &mut attacker.stats.vehicle_damage_by_type,
                vehicle_type.unwrap_or_else(|| "UNKNOWN".to_string()),
                damage,
            );
            add_f64(
                &mut attacker.stats.vehicle_damage_by_weapon,
                source.breakdown_key(),
                damage,
            );
            if is_fatal && self.fatal_vehicle_entities.insert(vehicle_entity_id) {
                attacker.stats.vehicle_final_hits += 1;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn vehicle_destroyed(
        &mut self,
        vehicle_entity_id: String,
        vehicle_type: Option<String>,
        attacker_id: Option<Uuid>,
        owner_id: Option<Uuid>,
        source: &DamageSource,
        reported_teamkill: bool,
        line: u64,
    ) {
        if !self
            .destroyed_vehicle_entities
            .insert(vehicle_entity_id.clone())
        {
            self.warn(format!(
                "line {line}: duplicate vehicle_destroyed for vehicleEntityId {vehicle_entity_id} ignored"
            ));
            return;
        }
        let vehicle_type = vehicle_type.unwrap_or_else(|| "UNKNOWN".to_string());
        if vehicle_type == "EMPTY" {
            return;
        }
        if let Some(owner_id) = owner_id {
            self.ensure_player(owner_id, line);
            if let Some(owner) = self.players.get_mut(&owner_id) {
                owner.stats.vehicles_lost += 1;
            }
        }
        let Some(attacker_id) = attacker_id else {
            return;
        };
        self.ensure_player(attacker_id, line);
        let inferred_teamkill = owner_id
            .filter(|owner_id| *owner_id != attacker_id)
            .and_then(|owner_id| self.team_relation(attacker_id, owner_id));
        if let Some(inferred_teamkill) = inferred_teamkill
            && inferred_teamkill != reported_teamkill
        {
            self.warn(format!(
                "line {line}: vehicle teamkill flag ({reported_teamkill}) differs from team state ({inferred_teamkill})"
            ));
        }
        let is_teamkill = reported_teamkill || inferred_teamkill.unwrap_or(false);
        if let Some(attacker) = self.players.get_mut(&attacker_id) {
            if is_teamkill {
                attacker.stats.vehicle_teamkills += 1;
            } else {
                attacker.stats.vehicle_destructions += 1;
                *attacker
                    .stats
                    .vehicle_kills_by_type
                    .entry(vehicle_type)
                    .or_default() += 1;
                *attacker
                    .stats
                    .vehicle_kills_by_weapon
                    .entry(source.breakdown_key())
                    .or_default() += 1;
            }
        }
    }

    fn award_assists(&mut self, victim_id: Uuid, killer_id: Uuid) {
        let Some(damage) = self.damage_context.get(&victim_id) else {
            return;
        };
        let total: f64 = damage.values().sum();
        if total <= 0.0 {
            return;
        }
        let assistants = damage
            .iter()
            .filter_map(|(player_id, dealt)| {
                (*player_id != killer_id
                    && *player_id != victim_id
                    && *dealt / total >= ASSIST_THRESHOLD)
                    .then_some(*player_id)
            })
            .collect::<Vec<_>>();
        for player_id in assistants {
            if let Some(player) = self.players.get_mut(&player_id) {
                player.stats.assists += 1;
            }
        }
    }

    fn same_team(&self, left: Uuid, right: Uuid) -> bool {
        self.team_relation(left, right) == Some(true)
    }

    fn team_relation(&self, left: Uuid, right: Uuid) -> Option<bool> {
        let left = self
            .players
            .get(&left)
            .and_then(|player| valid_team(player.current_team.as_deref()));
        let right = self
            .players
            .get(&right)
            .and_then(|player| valid_team(player.current_team.as_deref()));
        match (left, right) {
            (Some(left), Some(right)) => Some(left == right),
            _ => None,
        }
    }

    fn ensure_player(&mut self, player_id: Uuid, line: u64) {
        if let std::collections::hash_map::Entry::Vacant(entry) = self.players.entry(player_id) {
            entry.insert(PlayerState::default());
            if self.unknown_player_warnings.insert(player_id) {
                self.warn(format!(
                    "line {line}: player {player_id} was referenced before player_joined"
                ));
            }
        }
    }

    fn warn(&mut self, warning: String) {
        const MAX_STORED_WARNINGS: usize = 500;
        if self.warnings.len() < MAX_STORED_WARNINGS {
            self.warnings.push(warning);
        } else if self.warnings.len() == MAX_STORED_WARNINGS {
            self.warnings
                .push("additional warnings were truncated".to_string());
        }
    }
}

fn close_presence(player: &mut PlayerState, end: i64, warnings: &mut Vec<String>) {
    if let Some(joined_at) = player.active_since.take() {
        if end >= joined_at {
            player.time_in_game_ms = player.time_in_game_ms.saturating_add(end - joined_at);
        } else if warnings.len() < 500 {
            warnings.push("active player interval starts after game_ended".to_string());
        }
    }
}

fn valid_team(team: Option<&str>) -> Option<&str> {
    team.filter(|team| !team.is_empty() && *team != "Spectators")
}

fn fallback_nickname(player_id: Uuid) -> String {
    let compact = player_id.as_simple().to_string();
    format!("Unknown-{}", &compact[..8])
}

fn add_f64(map: &mut BTreeMap<String, f64>, key: String, value: f64) {
    *map.entry(key).or_default() += value;
}
