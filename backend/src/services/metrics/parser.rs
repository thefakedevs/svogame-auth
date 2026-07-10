use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::aggregation::{AggregationError, MatchAggregate, TimelineAggregator};
use super::model::{ParseError, parse_event};

#[derive(Debug, thiserror::Error)]
pub enum TimelineStreamError {
    #[error("timeline exceeds the configured {max_bytes} byte limit")]
    TooLarge { max_bytes: usize },
    #[error("NDJSON line exceeds the configured upload limit")]
    LineTooLarge,
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error(transparent)]
    Aggregate(#[from] AggregationError),
}

pub struct ParsedTimeline {
    pub aggregate: MatchAggregate,
    pub content_sha256: String,
    pub byte_count: usize,
    pub line_count: u64,
}

pub struct TimelineStream {
    aggregator: TimelineAggregator,
    hasher: Sha256,
    pending_line: Vec<u8>,
    byte_count: usize,
    line_count: u64,
    max_bytes: usize,
}

impl TimelineStream {
    pub fn new(game_id: Uuid, max_bytes: usize) -> Self {
        Self {
            aggregator: TimelineAggregator::new(game_id),
            hasher: Sha256::new(),
            pending_line: Vec::new(),
            byte_count: 0,
            line_count: 0,
            max_bytes,
        }
    }

    pub fn push_chunk(&mut self, chunk: &[u8]) -> Result<(), TimelineStreamError> {
        self.byte_count = self
            .byte_count
            .checked_add(chunk.len())
            .filter(|size| *size <= self.max_bytes)
            .ok_or(TimelineStreamError::TooLarge {
                max_bytes: self.max_bytes,
            })?;
        self.hasher.update(chunk);
        let mut start = 0;
        for (index, byte) in chunk.iter().enumerate() {
            if *byte != b'\n' {
                continue;
            }
            self.pending_line.extend_from_slice(&chunk[start..index]);
            self.process_pending_line()?;
            start = index + 1;
        }
        self.pending_line.extend_from_slice(&chunk[start..]);
        if self.pending_line.len() > self.max_bytes {
            return Err(TimelineStreamError::LineTooLarge);
        }
        Ok(())
    }

    pub fn finish(mut self) -> Result<ParsedTimeline, TimelineStreamError> {
        if !self.pending_line.is_empty() {
            self.process_pending_line()?;
        }
        let hash = self.hasher.finalize();
        Ok(ParsedTimeline {
            aggregate: self.aggregator.finish()?,
            content_sha256: format!("{hash:x}"),
            byte_count: self.byte_count,
            line_count: self.line_count,
        })
    }

    fn process_pending_line(&mut self) -> Result<(), TimelineStreamError> {
        self.line_count += 1;
        if self.pending_line.last() == Some(&b'\r') {
            self.pending_line.pop();
        }
        if let Some(event) = parse_event(&self.pending_line, self.line_count)? {
            self.aggregator.apply(event, self.line_count)?;
        }
        self.pending_line.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAME: &str = "00000000-0000-0000-0000-000000000001";
    const OTHER_GAME: &str = "00000000-0000-0000-0000-000000000099";
    const A: &str = "00000000-0000-0000-0000-000000000002";
    const B: &str = "00000000-0000-0000-0000-000000000003";
    const C: &str = "00000000-0000-0000-0000-000000000004";

    fn parse(body: &str) -> Result<ParsedTimeline, TimelineStreamError> {
        let game_id = Uuid::parse_str(GAME).expect("valid fixture game UUID");
        let mut stream = TimelineStream::new(game_id, 1024 * 1024);
        for chunk in body.as_bytes().chunks(7) {
            stream.push_chunk(chunk)?;
        }
        stream.finish()
    }

    fn player<'a>(
        timeline: &'a ParsedTimeline,
        id: &str,
    ) -> &'a super::super::aggregation::PlayerAggregate {
        let id = Uuid::parse_str(id).expect("valid fixture player UUID");
        timeline
            .aggregate
            .players
            .iter()
            .find(|player| player.player_id == id)
            .expect("player aggregate")
    }

    fn event(kind: &str, timestamp: i64, fields: &str) -> String {
        format!(r#"{{"type":"{kind}","timestampMs":{timestamp},{fields}}}"#)
    }

    fn start() -> String {
        event(
            "game_started",
            0,
            &format!(r#""gameId":"{GAME}","map":"svo""#),
        )
    }

    fn end(timestamp: i64) -> String {
        event(
            "game_ended",
            timestamp,
            &format!(r#""gameId":"{GAME}","winningTeam":"Attack""#),
        )
    }

    fn join(id: &str, nickname: &str, team: &str, timestamp: i64) -> String {
        event(
            "player_joined",
            timestamp,
            &format!(r#""playerId":"{id}","nickname":"{nickname}","team":"{team}""#),
        )
    }

    fn damage(
        victim: &str,
        attacker: Option<&str>,
        amount: f64,
        timestamp: i64,
        headshot: bool,
    ) -> String {
        let attacker = attacker
            .map(|value| format!(r#""{value}""#))
            .unwrap_or_else(|| "null".to_string());
        event(
            "player_damage",
            timestamp,
            &format!(
                r#""victimId":"{victim}","attackerId":{attacker},"damage":{amount},"source":{{"type":"tacz:gun","weaponId":"tacz:m4a1"}},"headshot":{headshot}"#
            ),
        )
    }

    fn death(victim: &str, killer: Option<&str>, timestamp: i64) -> String {
        let killer = killer
            .map(|value| format!(r#""{value}""#))
            .unwrap_or_else(|| "null".to_string());
        event(
            "player_death",
            timestamp,
            &format!(
                r#""victimId":"{victim}","killerId":{killer},"totalDamage":100.0,"source":{{"type":"tacz:gun","weaponId":"tacz:m4a1"}}"#
            ),
        )
    }

    #[test]
    fn calculates_kills_assists_at_threshold_and_clears_context() {
        let body = [
            start(),
            join(A, "A", "Attack", 1),
            join(B, "B", "Defense", 1),
            join(C, "C", "Attack", 1),
            damage(B, Some(A), 70.0, 2, false),
            damage(B, Some(C), 30.0, 3, false),
            death(B, Some(A), 4),
            damage(B, Some(A), 71.0, 5, false),
            damage(B, Some(C), 29.0, 6, false),
            death(B, Some(A), 7),
            death(B, Some(A), 8),
            end(10),
        ]
        .join("\n");
        let timeline = parse(&body).expect("valid timeline");
        assert_eq!(player(&timeline, A).stats.kills, 3);
        assert_eq!(player(&timeline, A).stats.assists, 0);
        assert_eq!(player(&timeline, B).stats.deaths, 3);
        assert_eq!(player(&timeline, B).stats.assists, 0);
        assert_eq!(player(&timeline, C).stats.assists, 1);
    }

    #[test]
    fn distinguishes_teamkill_suicide_and_environment_death() {
        let body = [
            start(),
            join(A, "A", "Attack", 1),
            join(B, "B", "Attack", 1),
            death(B, Some(A), 2),
            death(B, None, 3),
            death(A, Some(A), 4),
            end(5),
        ]
        .join("\n");
        let timeline = parse(&body).expect("valid timeline");
        assert_eq!(player(&timeline, A).stats.kills, 0);
        assert_eq!(player(&timeline, A).stats.teamkills, 1);
        assert_eq!(player(&timeline, A).stats.suicides, 1);
        assert_eq!(player(&timeline, A).stats.environment_deaths, 0);
        assert_eq!(player(&timeline, B).stats.suicides, 1);
        assert_eq!(player(&timeline, B).stats.environment_deaths, 1);
    }

    #[test]
    fn tracks_team_changes_friendly_self_damage_and_headshots() {
        let body = [
            start(),
            join(A, "A", "Attack", 1),
            join(B, "B", "Defense", 1),
            damage(A, Some(A), 4.0, 2, true),
            damage(B, Some(A), 5.0, 3, false),
            event(
                "player_team_changed",
                4,
                &format!(r#""playerId":"{A}","previousTeam":"Attack","newTeam":"Defense""#),
            ),
            damage(B, Some(A), 3.0, 5, false),
            end(6),
        ]
        .join("\n");
        let timeline = parse(&body).expect("valid timeline");
        let player = player(&timeline, A);
        assert_eq!(player.stats.damage_dealt, 12.0);
        assert_eq!(player.stats.self_damage, 4.0);
        assert_eq!(player.stats.friendly_damage, 3.0);
        assert_eq!(player.stats.headshots, 1);
        assert_eq!(player.stats.headshot_damage, 4.0);
        assert!(player.changed_team);
        assert_eq!(player.team_changes, 1);
    }

    #[test]
    fn deduplicates_vehicle_events_and_excludes_empty_and_teamkills() {
        let vehicle_damage = |entity: &str, fatal: bool, timestamp: i64| {
            event(
                "vehicle_damage",
                timestamp,
                &format!(
                    r#""vehicleId":"vvp:car","vehicleEntityId":"{entity}","vehicleType":"CAR","vehicleHealth":0.0,"vehicleMaxHealth":100.0,"attackerId":"{A}","vehicleOwnerId":"{B}","damage":10.0,"source":{{"type":"gunfire","weaponId":"gun"}},"isFatal":{fatal}"#
                ),
            )
        };
        let destroyed = |entity: &str, vehicle_type: &str, owner: &str, timestamp: i64| {
            event(
                "vehicle_destroyed",
                timestamp,
                &format!(
                    r#""vehicleId":"vvp:car","vehicleEntityId":"{entity}","vehicleType":"{vehicle_type}","vehicleMaxHealth":100.0,"attackerId":"{A}","vehicleOwnerId":"{owner}","finalDamage":10.0,"source":{{"type":"gunfire","weaponId":"gun"}},"isTeamKill":false"#
                ),
            )
        };
        let body = [
            start(),
            join(A, "A", "Attack", 1),
            join(B, "B", "Defense", 1),
            join(C, "C", "Attack", 1),
            vehicle_damage("vehicle-1", true, 2),
            vehicle_damage("vehicle-1", true, 3),
            destroyed("vehicle-1", "CAR", B, 4),
            destroyed("vehicle-1", "CAR", B, 5),
            destroyed("empty", "EMPTY", B, 6),
            destroyed("friendly", "CAR", C, 7),
            end(8),
        ]
        .join("\n");
        let timeline = parse(&body).expect("valid timeline");
        let player = player(&timeline, A);
        assert_eq!(player.stats.vehicle_final_hits, 1);
        assert_eq!(player.stats.vehicle_destructions, 1);
        assert_eq!(player.stats.vehicle_teamkills, 1);
        assert_eq!(player.stats.vehicle_damage_dealt, 20.0);
        assert!(
            timeline
                .aggregate
                .warnings
                .iter()
                .any(|warning| warning.contains("duplicate vehicle_destroyed"))
        );
    }

    #[test]
    fn handles_rejoin_missing_left_nickname_changes_equal_timestamps_and_unknown_events() {
        let body = [
            start(),
            join(A, "First", "Attack", 10),
            event(
                "player_left",
                20,
                &format!(r#""playerId":"{A}","reason":null"#),
            ),
            join(A, "Second", "Attack", 30),
            join(B, "B", "Defense", 30),
            event("future_event", 30, r#""value":1"#),
            end(50),
        ]
        .join("\n");
        let timeline = parse(&body).expect("valid timeline");
        let a = player(&timeline, A);
        assert_eq!(a.time_in_game_ms, 30);
        assert_eq!(a.left_count, 1);
        assert_eq!(a.nickname, "Second");
        assert_eq!(a.nicknames, vec!["First".to_string(), "Second".to_string()]);
        assert_eq!(player(&timeline, B).time_in_game_ms, 20);
        assert!(
            timeline
                .aggregate
                .warnings
                .iter()
                .any(|warning| warning.contains("unknown event type"))
        );
    }

    #[test]
    fn accepts_empty_lines_extra_fields_and_utf8_bom() {
        let body = format!(
            "\u{feff}{}\n\n{}\n{}",
            start(),
            event(
                "player_joined",
                1,
                &format!(r#""playerId":"{A}","nickname":"Игрок","team":"Attack","extra":true"#),
            ),
            end(2)
        );
        let timeline = parse(&body).expect("valid timeline");
        assert_eq!(timeline.aggregate.event_count, 3);
        assert_eq!(player(&timeline, A).nickname, "Игрок");
    }

    #[test]
    fn rejects_invalid_json_missing_fields_mismatched_game_and_non_utf8() {
        assert!(parse(&format!("{}\nnot-json", start())).is_err());
        assert!(
            parse(&format!(
                "{}\n{}",
                start(),
                event(
                    "player_joined",
                    1,
                    &format!(r#""playerId":"{A}","team":"Attack""#)
                )
            ))
            .is_err()
        );
        assert!(
            parse(&format!(
                "{}\n{}",
                start(),
                event(
                    "game_ended",
                    2,
                    &format!(r#""gameId":"{OTHER_GAME}","winningTeam":null"#)
                )
            ))
            .is_err()
        );

        let game_id = Uuid::parse_str(GAME).expect("valid fixture game UUID");
        let mut stream = TimelineStream::new(game_id, 1024);
        assert!(stream.push_chunk(&[0xff, b'\n']).is_err());
    }

    #[test]
    fn enforces_upload_limit() {
        let game_id = Uuid::parse_str(GAME).expect("valid fixture game UUID");
        let mut stream = TimelineStream::new(game_id, 8);
        assert!(matches!(
            stream.push_chunk(b"123456789"),
            Err(TimelineStreamError::TooLarge { .. })
        ));
    }
}
