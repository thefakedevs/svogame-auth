import { useEffect, useState } from 'react'
import { getMatch, getPlayerMatches, type MatchResponse, type PlayerMatchesResponse } from '../../api/metrics'
import { ApiError, toDisplayError } from '../../api/http'
import PlayerHead from '../metrics/PlayerHead'
import AppPortal from '../../shared/ui/portal/AppPortal'
import ProfileSkeleton from './ProfileSkeleton'

const numberFormatter = new Intl.NumberFormat('ru-RU', { maximumFractionDigits: 2 })
const dateFormatter = new Intl.DateTimeFormat('ru-RU', { day: '2-digit', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' })

function formatDuration(value: number | null) {
  if (value === null) return 'Не завершён'
  const hours = Math.floor(value / 3_600_000)
  const minutes = Math.floor((value % 3_600_000) / 60_000)
  const seconds = Math.floor((value % 60_000) / 1000)
  return hours ? `${hours} ч ${minutes} мин` : `${minutes} мин ${seconds} сек`
}

function MatchLoader({ small = false }: { small?: boolean }) {
  return (
    <span className={`ui-spinner ${small ? 'ui-spinner-sm' : ''}`} role="status" aria-label="Загрузка">
      <span className="ui-spinner-track" aria-hidden>
        <span className="ui-spinner-orb" />
        <span className="ui-spinner-orb" />
        <span className="ui-spinner-orb" />
        <span className="ui-spinner-orb" />
      </span>
    </span>
  )
}

function teamName(player: MatchResponse['players'][number]) {
  return player.finalTeam ?? player.initialTeam ?? 'Без команды'
}

function teamLabel(team: string | null) {
  if (!team) return 'Не определён'
  if (team.toLowerCase() === 'attack') return 'Атака'
  if (team.toLowerCase() === 'defense') return 'Защита'
  return team
}

function teamClass(team: string) {
  const normalized = team.toLowerCase()
  return normalized === 'attack' ? 'is-attack' : normalized === 'defense' ? 'is-defense' : ''
}

function teamOrder(name: string) {
  const normalized = name.toLowerCase()
  if (normalized === 'attack') return 0
  if (normalized === 'defense') return 1
  return 2
}

function isWinningTeam(team: string, winningTeam: string | null) {
  return team.toLowerCase() === winningTeam?.toLowerCase()
}

function matchOutcome(teamWon: boolean | null | undefined) {
  if (teamWon === true) return { label: 'Победа', className: 'is-won' }
  if (teamWon === false) return { label: 'Поражение', className: 'is-lost' }
  return { label: 'Неизвестно', className: 'is-unknown' }
}

function TeamPlayers({
  team,
  players,
  playerId,
  isWinner,
}: {
  team: string
  players: MatchResponse['players']
  playerId: string
  isWinner: boolean
}) {
  return (
    <section className={`profile-match-team ${teamClass(team)} ${isWinner ? 'is-winner' : ''}`}>
      <header className="profile-match-team-header">
        <div>
          <h3>{teamLabel(team)}</h3>
        </div>
        {isWinner ? <span className="ui-badge ui-badge-success">Победа</span> : null}
      </header>
      <div className="profile-match-players-wrap">
        <table className="profile-match-players">
          <thead><tr><th>Никнейм</th><th>K / D / A</th><th>Урон</th><th>Техника</th><th>K/D</th></tr></thead>
          <tbody>{players.map((player) => (
            <tr key={player.playerId} className={player.playerId === playerId ? 'is-current-player' : ''}>
              <td><span className="profile-match-player"><PlayerHead playerId={player.playerId} nickname={player.nickname} /><strong>{player.nickname}</strong></span></td>
              <td>{player.kills} / {player.deaths} / {player.assists}</td>
              <td>{numberFormatter.format(player.damageDealt)}</td><td>{player.vehicleDestructions}</td><td>{player.kd}</td>
            </tr>
          ))}</tbody>
        </table>
      </div>
    </section>
  )
}

function MatchDetails({ match, playerId, onClose }: { match: MatchResponse; playerId: string; onClose: () => void }) {
  const teams = Array.from(
    match.players.reduce((groups, player) => {
      const team = teamName(player)
      groups.set(team, [...(groups.get(team) ?? []), player])
      return groups
    }, new Map<string, MatchResponse['players']>()),
  )
    .map(([team, players]) => [
      team,
      [...players].sort(
        (left, right) =>
          right.damageDealt - left.damageDealt
          || right.kills - left.kills
          || left.nickname.localeCompare(right.nickname, 'ru'),
      ),
    ] as const)
    .sort(([left], [right]) => teamOrder(left) - teamOrder(right) || left.localeCompare(right, 'ru'))

  return (
    <AppPortal>
      <div className="ui-modal-backdrop profile-match-modal-backdrop" role="presentation" onMouseDown={onClose}>
        <section className="ui-modal profile-match-modal" role="dialog" aria-modal="true" aria-labelledby="profile-match-title" onMouseDown={(event) => event.stopPropagation()}>
          <div className="ui-modal-header">
            <div><span className="ui-section-title">Детали матча</span><h2 id="profile-match-title">{match.map}</h2></div>
            <button className="btn btn-sm" type="button" onClick={onClose} aria-label="Закрыть">×</button>
          </div>
          <div className="profile-match-meta">
            <span>{dateFormatter.format(new Date(match.startedAt))}</span><span>{formatDuration(match.durationMs)}</span>
          </div>
          <div className="profile-match-team-list">
            {teams.map(([team, players]) => <TeamPlayers key={team} team={team} players={players} playerId={playerId} isWinner={isWinningTeam(team, match.winningTeam)} />)}
          </div>
        </section>
      </div>
    </AppPortal>
  )
}

export default function ProfileMatchesTab({ playerId }: { playerId: string }) {
  const [offset, setOffset] = useState(0)
  const [data, setData] = useState<PlayerMatchesResponse | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [details, setDetails] = useState<MatchResponse | null>(null)
  const [detailsLoading, setDetailsLoading] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    setLoading(true)
    getPlayerMatches(playerId, { limit: 10, offset })
      .then((matches) => { if (!cancelled) { setData(matches); setError(null) } })
      .catch((reason) => {
        if (cancelled) return
        if (reason instanceof ApiError && reason.status === 404) {
          setData(null)
          setError(null)
          return
        }
        setError(toDisplayError(reason, 'Не удалось загрузить историю матчей.'))
      })
      .finally(() => { if (!cancelled) setLoading(false) })
    return () => { cancelled = true }
  }, [offset, playerId])

  const openMatch = async (gameId: string) => {
    setDetailsLoading(gameId)
    try { setDetails(await getMatch(gameId)) }
    catch (reason) { setError(toDisplayError(reason, 'Не удалось загрузить детали матча.')) }
    finally { setDetailsLoading(null) }
  }

  if (loading && !data) return <ProfileSkeleton activeTab="matches" />
  if (error && !data) return <div className="ui-alert ui-alert-error">{error}</div>

  const total = data?.pagination.total ?? 0
  return (
    <div className="profile-matches-tab">
      {error ? <div className="ui-alert ui-alert-error">{error}</div> : null}
      <section className="card profile-panel profile-match-history">
        <div className="ui-card-header"><div><h2 className="card-title">Ваши матчи</h2><p className="profile-subtle">Здесь собраны результаты всех сыгранных матчей.</p></div></div>
        {data?.matches.length ? <div className="profile-match-list">{data.matches.map((match) => (
          (() => {
            const outcome = matchOutcome(match.teamWon)
            return (
              <button className="profile-match-row" type="button" key={match.gameId} onClick={() => void openMatch(match.gameId)}>
                <span className={`profile-match-result ${outcome.className}`}>{outcome.label}</span>
                <span><strong>{match.map}</strong><small>{dateFormatter.format(new Date(match.startedAt))}</small></span>
                <span><small>Длительность</small><strong>{formatDuration(match.durationMs)}</strong></span>
                <span><small>Победитель</small><strong>{match.winningTeam ? teamLabel(match.winningTeam) : '—'}</strong></span>
                <span className="profile-match-open">{detailsLoading === match.gameId ? <><MatchLoader small /> Загрузка…</> : 'Подробнее →'}</span>
              </button>
            )
          })()
        ))}</div> : <div className="profile-empty"><p>Пока здесь пусто. Сыграйте первый матч — после него здесь появится ваша статистика.</p></div>}
        {total > 10 ? <div className="profile-match-pagination">
          <button className="btn btn-sm" type="button" disabled={offset === 0 || loading} onClick={() => setOffset(Math.max(0, offset - 10))}>Назад</button>
          <span>{offset + 1}–{Math.min(offset + 10, total)} из {total}</span>
          <button className="btn btn-sm" type="button" disabled={offset + 10 >= total || loading} onClick={() => setOffset(offset + 10)}>Дальше</button>
        </div> : null}
        {loading && data ? <div className="profile-match-loading-overlay"><MatchLoader /> Обновляем матчи…</div> : null}
      </section>
      {details ? <MatchDetails match={details} playerId={playerId} onClose={() => setDetails(null)} /> : null}
    </div>
  )
}
