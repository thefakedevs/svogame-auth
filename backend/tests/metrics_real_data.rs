use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use auth::services::metrics::TimelineStream;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default)]
struct CoreMetrics {
    kills: i64,
    deaths: i64,
    assists: i64,
    vehicle_destructions: i64,
}

#[test]
fn parses_all_real_timelines_and_reports_legacy_differences() {
    let Ok(root) = std::env::var("SVO_METRICS_FIXTURES_DIR").map(PathBuf::from) else {
        eprintln!("skipping real metrics regression: SVO_METRICS_FIXTURES_DIR is not set");
        return;
    };
    let files = timeline_files(&root);
    assert!(!files.is_empty(), "no timeline.ndjson fixtures found");

    let mut completed = 0;
    let mut incomplete = 0;
    let mut event_count = 0_u64;
    let mut warning_count = 0_usize;
    let mut legacy_total = CoreMetrics::default();
    let mut current_total = CoreMetrics::default();
    let mut per_file_differences = Vec::new();

    for path in &files {
        let bytes = std::fs::read(path).expect("read real timeline");
        let game_id = path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .and_then(|name| Uuid::parse_str(name).ok())
            .expect("fixture parent directory is a game UUID");
        let mut stream = TimelineStream::new(game_id, 16 * 1024 * 1024);
        for chunk in bytes.chunks(8192) {
            stream
                .push_chunk(chunk)
                .expect("stream real timeline chunk");
        }
        let parsed = stream.finish().expect("parse real timeline");
        event_count += parsed.aggregate.event_count;
        warning_count += parsed.aggregate.warnings.len();
        if parsed.aggregate.status == "completed" {
            completed += 1;
        } else {
            incomplete += 1;
        }

        let legacy = legacy_metrics(&bytes);
        let legacy_players = legacy.keys().cloned().collect::<HashSet<_>>();
        let current = parsed
            .aggregate
            .players
            .iter()
            .filter(|player| legacy_players.contains(&player.player_id.to_string()))
            .fold(CoreMetrics::default(), |mut totals, player| {
                totals.kills += player.stats.kills;
                totals.deaths += player.stats.deaths;
                totals.assists += player.stats.assists;
                totals.vehicle_destructions += player.stats.vehicle_destructions;
                totals
            });
        let legacy = legacy
            .values()
            .fold(CoreMetrics::default(), |mut totals, stats| {
                totals.kills += stats.kills;
                totals.deaths += stats.deaths;
                totals.assists += stats.assists;
                totals.vehicle_destructions += stats.vehicle_destructions;
                totals
            });
        add_core(&mut legacy_total, legacy);
        add_core(&mut current_total, current);
        if legacy.kills != current.kills
            || legacy.deaths != current.deaths
            || legacy.assists != current.assists
            || legacy.vehicle_destructions != current.vehicle_destructions
        {
            per_file_differences.push((game_id, legacy, current));
        }
    }

    println!(
        "real metrics regression: files={}, completed={}, incomplete={}, events={}, warnings={}",
        files.len(),
        completed,
        incomplete,
        event_count,
        warning_count
    );
    println!("legacy totals: {legacy_total:?}");
    println!("current totals: {current_total:?}");
    println!("files with differences: {}", per_file_differences.len());
    for (game_id, legacy, current) in per_file_differences.iter().take(25) {
        println!("{game_id}: legacy={legacy:?}, current={current:?}");
    }
    assert_eq!(completed + incomplete, files.len());
}

fn timeline_files(root: &Path) -> Vec<PathBuf> {
    let mut files = std::fs::read_dir(root)
        .expect("read fixtures directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("timeline.ndjson"))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn legacy_metrics(bytes: &[u8]) -> HashMap<String, CoreMetrics> {
    let mut players: HashMap<String, String> = HashMap::new();
    let mut metrics: HashMap<String, CoreMetrics> = HashMap::new();
    let mut damage: HashMap<String, HashMap<String, f64>> = HashMap::new();
    for line in bytes.split(|byte| *byte == b'\n') {
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let event: Value = serde_json::from_slice(line).expect("real fixture JSON");
        match event.get("type").and_then(Value::as_str) {
            Some("player_joined") => {
                let team = event
                    .get("team")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if !team.is_empty()
                    && team != "Spectators"
                    && let Some(player_id) = event.get("playerId").and_then(Value::as_str)
                {
                    players.insert(player_id.to_string(), team.to_string());
                    metrics.entry(player_id.to_string()).or_default();
                }
            }
            Some("player_team_changed") => {
                let player_id = event.get("playerId").and_then(Value::as_str);
                let new_team = event.get("newTeam").and_then(Value::as_str);
                if let (Some(player_id), Some(new_team)) = (player_id, new_team)
                    && players.contains_key(player_id)
                {
                    players.insert(player_id.to_string(), new_team.to_string());
                    damage.remove(player_id);
                }
            }
            Some("player_damage") => {
                let victim = event.get("victimId").and_then(Value::as_str);
                let attacker = event.get("attackerId").and_then(Value::as_str);
                let amount = event
                    .get("damage")
                    .and_then(Value::as_f64)
                    .unwrap_or_default();
                if let (Some(victim), Some(attacker)) = (victim, attacker)
                    && amount > 0.0
                {
                    *damage
                        .entry(victim.to_string())
                        .or_default()
                        .entry(attacker.to_string())
                        .or_default() += amount;
                }
            }
            Some("player_death") => {
                let victim = event.get("victimId").and_then(Value::as_str);
                let killer = event.get("killerId").and_then(Value::as_str);
                if let Some(victim) = victim {
                    if players.contains_key(victim) {
                        metrics.entry(victim.to_string()).or_default().deaths += 1;
                    }
                    if let Some(killer) = killer
                        && killer != victim
                        && players.contains_key(victim)
                        && players.contains_key(killer)
                        && players.get(victim) != players.get(killer)
                    {
                        metrics.entry(killer.to_string()).or_default().kills += 1;
                        if let Some(dealers) = damage.get(victim) {
                            let total: f64 = dealers.values().sum();
                            if total > 0.0 {
                                for (attacker, amount) in dealers {
                                    if attacker != killer
                                        && attacker != victim
                                        && players.contains_key(attacker)
                                        && *amount / total >= 0.30
                                    {
                                        metrics.entry(attacker.clone()).or_default().assists += 1;
                                    }
                                }
                            }
                        }
                    }
                    damage.remove(victim);
                }
            }
            Some("vehicle_destroyed") => {
                let attacker = event.get("attackerId").and_then(Value::as_str);
                let vehicle_type = event.get("vehicleType").and_then(Value::as_str);
                if let Some(attacker) = attacker
                    && players.contains_key(attacker)
                    && vehicle_type != Some("EMPTY")
                {
                    metrics
                        .entry(attacker.to_string())
                        .or_default()
                        .vehicle_destructions += 1;
                }
            }
            _ => {}
        }
    }
    metrics
}

fn add_core(target: &mut CoreMetrics, value: CoreMetrics) {
    target.kills += value.kills;
    target.deaths += value.deaths;
    target.assists += value.assists;
    target.vehicle_destructions += value.vehicle_destructions;
}
