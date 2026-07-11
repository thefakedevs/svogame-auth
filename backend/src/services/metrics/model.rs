use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

const MAX_GAME_NUMBER: f64 = f32::MAX as f64;

#[derive(Clone, Debug)]
pub struct DamageSource {
    pub kind: String,
    pub weapon_id: Option<String>,
}

impl DamageSource {
    pub fn breakdown_key(&self) -> String {
        self.weapon_id
            .clone()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| self.kind.clone())
    }
}

#[derive(Clone, Debug)]
pub enum TimelineEvent {
    GameStarted {
        timestamp_ms: i64,
        game_id: Uuid,
        map: String,
    },
    GameEnded {
        timestamp_ms: i64,
        game_id: Uuid,
        winning_team: Option<String>,
    },
    PlayerJoined {
        timestamp_ms: i64,
        player_id: Uuid,
        nickname: String,
        team: Option<String>,
    },
    PlayerLeft {
        timestamp_ms: i64,
        player_id: Uuid,
    },
    PlayerTeamChanged {
        timestamp_ms: i64,
        player_id: Uuid,
        new_team: String,
    },
    PlayerDamage {
        timestamp_ms: i64,
        victim_id: Uuid,
        attacker_id: Option<Uuid>,
        damage: f64,
        source: DamageSource,
        headshot: bool,
    },
    PlayerDeath {
        timestamp_ms: i64,
        victim_id: Uuid,
        killer_id: Option<Uuid>,
        source: DamageSource,
    },
    VehicleDamage {
        timestamp_ms: i64,
        vehicle_entity_id: String,
        vehicle_type: Option<String>,
        attacker_id: Option<Uuid>,
        vehicle_owner_id: Option<Uuid>,
        damage: f64,
        source: DamageSource,
        is_fatal: bool,
    },
    VehicleDestroyed {
        timestamp_ms: i64,
        vehicle_entity_id: String,
        vehicle_type: Option<String>,
        attacker_id: Option<Uuid>,
        vehicle_owner_id: Option<Uuid>,
        source: DamageSource,
        is_team_kill: bool,
    },
    Ignored {
        timestamp_ms: Option<i64>,
        event_type: String,
        known: bool,
    },
}

impl TimelineEvent {
    pub fn timestamp_ms(&self) -> Option<i64> {
        match self {
            Self::GameStarted { timestamp_ms, .. }
            | Self::GameEnded { timestamp_ms, .. }
            | Self::PlayerJoined { timestamp_ms, .. }
            | Self::PlayerLeft { timestamp_ms, .. }
            | Self::PlayerTeamChanged { timestamp_ms, .. }
            | Self::PlayerDamage { timestamp_ms, .. }
            | Self::PlayerDeath { timestamp_ms, .. }
            | Self::VehicleDamage { timestamp_ms, .. }
            | Self::VehicleDestroyed { timestamp_ms, .. } => Some(*timestamp_ms),
            Self::Ignored { timestamp_ms, .. } => *timestamp_ms,
        }
    }

    pub fn event_type(&self) -> &str {
        match self {
            Self::GameStarted { .. } => "game_started",
            Self::GameEnded { .. } => "game_ended",
            Self::PlayerJoined { .. } => "player_joined",
            Self::PlayerLeft { .. } => "player_left",
            Self::PlayerTeamChanged { .. } => "player_team_changed",
            Self::PlayerDamage { .. } => "player_damage",
            Self::PlayerDeath { .. } => "player_death",
            Self::VehicleDamage { .. } => "vehicle_damage",
            Self::VehicleDestroyed { .. } => "vehicle_destroyed",
            Self::Ignored { event_type, .. } => event_type,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("line {line}: invalid JSON: {detail}")]
    InvalidJson { line: u64, detail: String },
    #[error("line {line}: missing or invalid event type")]
    MissingType { line: u64 },
    #[error("line {line}: invalid {event_type} event: {detail}")]
    InvalidEvent {
        line: u64,
        event_type: String,
        detail: String,
    },
}

pub fn parse_event(bytes: &[u8], line: u64) -> Result<Option<TimelineEvent>, ParseError> {
    let bytes = strip_utf8_bom(bytes);
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return Ok(None);
    }
    let value: Value = serde_json::from_slice(bytes).map_err(|error| ParseError::InvalidJson {
        line,
        detail: error.to_string(),
    })?;
    let event_type_name = value
        .get("type")
        .and_then(Value::as_str)
        .ok_or(ParseError::MissingType { line })?
        .to_string();
    let event_type = event_type_name.as_str();
    let parsed = match event_type {
        "game_started" => {
            let raw: GameStarted = decode(value, line, event_type)?;
            TimelineEvent::GameStarted {
                timestamp_ms: timestamp(raw.timestamp_ms, line, event_type)?,
                game_id: uuid(&raw.game_id, line, event_type, "gameId")?,
                map: non_empty(raw.map, line, event_type, "map")?,
            }
        }
        "game_ended" => {
            let raw: GameEnded = decode(value, line, event_type)?;
            TimelineEvent::GameEnded {
                timestamp_ms: timestamp(raw.timestamp_ms, line, event_type)?,
                game_id: uuid(&raw.game_id, line, event_type, "gameId")?,
                winning_team: normalize_optional(raw.winning_team),
            }
        }
        "player_joined" => {
            let raw: PlayerJoined = decode(value, line, event_type)?;
            TimelineEvent::PlayerJoined {
                timestamp_ms: timestamp(raw.timestamp_ms, line, event_type)?,
                player_id: uuid(&raw.player_id, line, event_type, "playerId")?,
                nickname: non_empty(raw.nickname, line, event_type, "nickname")?,
                team: normalize_optional(raw.team),
            }
        }
        "player_left" => {
            let raw: PlayerLeft = decode(value, line, event_type)?;
            TimelineEvent::PlayerLeft {
                timestamp_ms: timestamp(raw.timestamp_ms, line, event_type)?,
                player_id: uuid(&raw.player_id, line, event_type, "playerId")?,
            }
        }
        "player_team_changed" => {
            let raw: PlayerTeamChanged = decode(value, line, event_type)?;
            TimelineEvent::PlayerTeamChanged {
                timestamp_ms: timestamp(raw.timestamp_ms, line, event_type)?,
                player_id: uuid(&raw.player_id, line, event_type, "playerId")?,
                new_team: non_empty(raw.new_team, line, event_type, "newTeam")?,
            }
        }
        "player_damage" => {
            let raw: PlayerDamage = decode(value, line, event_type)?;
            TimelineEvent::PlayerDamage {
                timestamp_ms: timestamp(raw.timestamp_ms, line, event_type)?,
                victim_id: uuid(&raw.victim_id, line, event_type, "victimId")?,
                attacker_id: optional_uuid(raw.attacker_id, line, event_type, "attackerId")?,
                damage: number(raw.damage, line, event_type, "damage")?,
                source: source(raw.source, line, event_type)?,
                headshot: raw.headshot,
            }
        }
        "player_death" => {
            let raw: PlayerDeath = decode(value, line, event_type)?;
            let _ = number(raw.total_damage, line, event_type, "totalDamage")?;
            TimelineEvent::PlayerDeath {
                timestamp_ms: timestamp(raw.timestamp_ms, line, event_type)?,
                victim_id: uuid(&raw.victim_id, line, event_type, "victimId")?,
                killer_id: optional_uuid(raw.killer_id, line, event_type, "killerId")?,
                source: source(raw.source, line, event_type)?,
            }
        }
        "vehicle_damage" => {
            let raw: VehicleDamage = decode(value, line, event_type)?;
            let _ = positive(raw.vehicle_max_health, line, event_type, "vehicleMaxHealth")?;
            let _ = number(raw.vehicle_health, line, event_type, "vehicleHealth")?;
            TimelineEvent::VehicleDamage {
                timestamp_ms: timestamp(raw.timestamp_ms, line, event_type)?,
                vehicle_entity_id: non_empty(
                    raw.vehicle_entity_id,
                    line,
                    event_type,
                    "vehicleEntityId",
                )?,
                vehicle_type: normalize_optional(raw.vehicle_type),
                attacker_id: optional_uuid(raw.attacker_id, line, event_type, "attackerId")?,
                vehicle_owner_id: optional_uuid(
                    raw.vehicle_owner_id,
                    line,
                    event_type,
                    "vehicleOwnerId",
                )?,
                damage: number(raw.damage, line, event_type, "damage")?,
                source: source(raw.source, line, event_type)?,
                is_fatal: raw.is_fatal,
            }
        }
        "vehicle_destroyed" => {
            let raw: VehicleDestroyed = decode(value, line, event_type)?;
            let _ = positive(raw.vehicle_max_health, line, event_type, "vehicleMaxHealth")?;
            let _ = number(raw.final_damage, line, event_type, "finalDamage")?;
            TimelineEvent::VehicleDestroyed {
                timestamp_ms: timestamp(raw.timestamp_ms, line, event_type)?,
                vehicle_entity_id: non_empty(
                    raw.vehicle_entity_id,
                    line,
                    event_type,
                    "vehicleEntityId",
                )?,
                vehicle_type: normalize_optional(raw.vehicle_type),
                attacker_id: optional_uuid(raw.attacker_id, line, event_type, "attackerId")?,
                vehicle_owner_id: optional_uuid(
                    raw.vehicle_owner_id,
                    line,
                    event_type,
                    "vehicleOwnerId",
                )?,
                source: source(raw.source, line, event_type)?,
                is_team_kill: raw.is_team_kill,
            }
        }
        "money_movement" => TimelineEvent::Ignored {
            timestamp_ms: value.get("timestampMs").and_then(Value::as_i64),
            event_type: event_type.to_string(),
            known: true,
        },
        _ => TimelineEvent::Ignored {
            timestamp_ms: value.get("timestampMs").and_then(Value::as_i64),
            event_type: event_type.to_string(),
            known: false,
        },
    };
    Ok(Some(parsed))
}

fn decode<T: for<'de> Deserialize<'de>>(
    value: Value,
    line: u64,
    event_type: &str,
) -> Result<T, ParseError> {
    serde_json::from_value(value).map_err(|error| invalid(line, event_type, error.to_string()))
}

fn uuid(value: &str, line: u64, event_type: &str, field: &str) -> Result<Uuid, ParseError> {
    Uuid::parse_str(value).map_err(|_| invalid(line, event_type, format!("{field} must be a UUID")))
}

fn optional_uuid(
    value: Option<String>,
    line: u64,
    event_type: &str,
    field: &str,
) -> Result<Option<Uuid>, ParseError> {
    value
        .map(|value| uuid(&value, line, event_type, field))
        .transpose()
}

fn timestamp(value: i64, line: u64, event_type: &str) -> Result<i64, ParseError> {
    if value < 0 {
        return Err(invalid(
            line,
            event_type,
            "timestampMs must be non-negative".to_string(),
        ));
    }
    Ok(value)
}

fn number(value: f64, line: u64, event_type: &str, field: &str) -> Result<f64, ParseError> {
    if !value.is_finite() || !(0.0..=MAX_GAME_NUMBER).contains(&value) {
        return Err(invalid(
            line,
            event_type,
            format!("{field} must be finite, non-negative and within f32 range"),
        ));
    }
    Ok(value)
}

fn positive(value: f64, line: u64, event_type: &str, field: &str) -> Result<f64, ParseError> {
    let value = number(value, line, event_type, field)?;
    if value == 0.0 {
        return Err(invalid(
            line,
            event_type,
            format!("{field} must be positive"),
        ));
    }
    Ok(value)
}

fn source(
    source: RawDamageSource,
    line: u64,
    event_type: &str,
) -> Result<DamageSource, ParseError> {
    Ok(DamageSource {
        kind: non_empty(source.kind, line, event_type, "source.type")?,
        weapon_id: normalize_optional(source.weapon_id),
    })
}

fn non_empty(
    value: String,
    line: u64,
    event_type: &str,
    field: &str,
) -> Result<String, ParseError> {
    if value.trim().is_empty() {
        return Err(invalid(
            line,
            event_type,
            format!("{field} must not be empty"),
        ));
    }
    Ok(value)
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
}

fn invalid(line: u64, event_type: &str, detail: String) -> ParseError {
    ParseError::InvalidEvent {
        line,
        event_type: event_type.to_string(),
        detail,
    }
}

fn strip_utf8_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GameStarted {
    timestamp_ms: i64,
    game_id: String,
    map: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GameEnded {
    timestamp_ms: i64,
    game_id: String,
    winning_team: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlayerJoined {
    timestamp_ms: i64,
    player_id: String,
    nickname: String,
    team: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlayerLeft {
    timestamp_ms: i64,
    player_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlayerTeamChanged {
    timestamp_ms: i64,
    player_id: String,
    new_team: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlayerDamage {
    timestamp_ms: i64,
    victim_id: String,
    attacker_id: Option<String>,
    damage: f64,
    source: RawDamageSource,
    #[serde(default)]
    headshot: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlayerDeath {
    timestamp_ms: i64,
    victim_id: String,
    killer_id: Option<String>,
    total_damage: f64,
    source: RawDamageSource,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VehicleDamage {
    timestamp_ms: i64,
    vehicle_entity_id: String,
    vehicle_type: Option<String>,
    vehicle_health: f64,
    vehicle_max_health: f64,
    attacker_id: Option<String>,
    vehicle_owner_id: Option<String>,
    damage: f64,
    source: RawDamageSource,
    #[serde(default)]
    is_fatal: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VehicleDestroyed {
    timestamp_ms: i64,
    vehicle_entity_id: String,
    vehicle_type: Option<String>,
    vehicle_max_health: f64,
    attacker_id: Option<String>,
    vehicle_owner_id: Option<String>,
    final_damage: f64,
    source: RawDamageSource,
    #[serde(default)]
    is_team_kill: bool,
}

#[derive(Deserialize)]
struct RawDamageSource {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "weaponId")]
    weapon_id: Option<String>,
}
