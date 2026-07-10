import { request } from './http'

export type MetricsPeriod = 'day' | 'week' | 'month' | 'all'
export type LeaderboardMetric =
  | 'kills'
  | 'deaths'
  | 'assists'
  | 'vehicle_destructions'
  | 'kd'
  | 'kda'
  | 'damage_dealt'
  | 'damage_per_minute'
  | 'win_rate'
  | 'headshots'
  | 'time_in_game'

export interface PaginationResponse {
  limit: number
  offset: number
  total: number
}

export interface PlayerStats {
  kills: number
  deaths: number
  assists: number
  teamkills: number
  suicides: number
  environmentDeaths: number
  damageDealt: number
  damageTaken: number
  friendlyDamage: number
  selfDamage: number
  headshots: number
  headshotDamage: number
  vehicleDamageDealt: number
  vehicleDamageTaken: number
  friendlyVehicleDamage: number
  vehicleFinalHits: number
  vehicleDestructions: number
  vehicleTeamkills: number
  vehiclesLost: number
  matchesPlayed: number
  wins: number
  losses: number
  killsByWeapon: Record<string, number>
  deathsBySource: Record<string, number>
  damageByWeapon: Record<string, number>
  damageBySource: Record<string, number>
  vehicleDamageByType: Record<string, number>
  vehicleDamageByWeapon: Record<string, number>
  vehicleKillsByType: Record<string, number>
  vehicleKillsByWeapon: Record<string, number>
}

export interface LeaderboardEntry {
  rank: number
  playerId: string
  nickname: string
  metricValue: number
  matchesPlayed: number
  kills: number
  deaths: number
  assists: number
  vehicleDestructions: number
  timeInGameMs: number
  kd: number
  kda: number
  damageDealt: number
  damagePerMinute: number
  winRate: number
  headshots: number
}

export interface LeaderboardResponse {
  metric: LeaderboardMetric
  pagination: PaginationResponse
  entries: LeaderboardEntry[]
}

export interface PlayerStatsResponse extends PlayerStats {
  playerId: string
  nickname: string
  timeInGameMs: number
  kd: number
  kda: number
  damagePerMinute: number
  winRate: number
}

export interface MatchListItem {
  gameId: string
  map: string
  startedAt: string
  endedAt: string | null
  durationMs: number | null
  winningTeam: string | null
  status: string
}

export interface PlayerMatchesResponse {
  playerId: string
  pagination: PaginationResponse
  matches: MatchListItem[]
}

export interface MatchPlayer extends PlayerStats {
  playerId: string
  nickname: string
  initialTeam: string | null
  finalTeam: string | null
  changedTeam: boolean
  teamChanges: number
  leftCount: number
  timeInGameMs: number
  kd: number
  kda: number
  damagePerMinute: number
}

export interface MatchResponse extends MatchListItem {
  eventsProcessed: number
  playerCount: number
  teams: Array<{
    team: string
    players: number
    kills: number
    deaths: number
    assists: number
    vehicleDestructions: number
  }>
  players: MatchPlayer[]
}

function queryString(values: Record<string, string | number | undefined>) {
  const query = new URLSearchParams()
  Object.entries(values).forEach(([key, value]) => {
    if (value !== undefined) query.set(key, String(value))
  })
  return query.toString()
}

export function getLeaderboard(options: {
  metric: LeaderboardMetric
  period: MetricsPeriod
  limit?: number
  offset?: number
  minMatches?: number
}) {
  return request<LeaderboardResponse>(`/api/metrics/leaderboard?${queryString(options)}`)
}

export function getPlayerStats(playerId: string, period: MetricsPeriod = 'all') {
  return request<PlayerStatsResponse>(`/api/metrics/players/${encodeURIComponent(playerId)}/stats?period=${period}`)
}

export function getPlayerMatches(playerId: string, options: { limit?: number; offset?: number } = {}) {
  return request<PlayerMatchesResponse>(
    `/api/metrics/players/${encodeURIComponent(playerId)}/matches?${queryString({ period: 'all', ...options })}`,
  )
}

export function getMatch(gameId: string) {
  return request<MatchResponse>(`/api/metrics/matches/${encodeURIComponent(gameId)}`)
}
