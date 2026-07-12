import { useEffect, useState } from 'react'
import { getMatch, getPlayerMatches, type MatchResponse, type PlayerMatchesResponse } from '../../api/metrics'
import { ApiError, toDisplayError } from '../../api/http'
import PlayerHead from '../metrics/PlayerHead'
import AppPortal from '../../shared/ui/portal/AppPortal'

const numberFormatter = new Intl.NumberFormat('ru-RU', { maximumFractionDigits: 2 })
const dateFormatter = new Intl.DateTimeFormat('ru-RU', {
  day: '2-digit',
  month: 'short',
  year: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
})

function formatDuration(value: number | null) {
  if (value === null) return 'Не завершён'
  const hours = Math.floor(value / 3_600_000)
  const minutes = Math.floor((value % 3_600_000) / 60_000)
  return hours ? `${hours} ч ${minutes} мин` : `${minutes} мин`
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

function teamOrder(team: string) {
  const normalized = team.toLowerCase()
  if (normalized === 'attack') return 0
  if (normalized === 'defense') return 1
  return 2
}

function outcome(teamWon: boolean | null | undefined) {
  if (teamWon === true) return { label: 'Победа', className: 'is-won' }
  if (teamWon === false) return { label: 'Поражение', className: 'is-lost' }
  return { label: 'Без итога', className: 'is-unknown' }
}

function MatchDetailsModal({ match, playerId, onClose }: { match: MatchResponse; playerId: string; onClose: () => void }) {
  const teams = Array.from(
    match.players.reduce((groups, player) => {
      const team = player.finalTeam ?? player.initialTeam ?? 'Без команды'
      groups.set(team, [...(groups.get(team) ?? []), player])
      return groups
    }, new Map<string, MatchResponse['players']>()),
  )
    .map(([team, players]) => [
      team,
      [...players].sort(
        (left, right) => right.damageDealt - left.damageDealt || right.kills - left.kills || left.nickname.localeCompare(right.nickname, 'ru'),
      ),
    ] as const)
    .sort(([left], [right]) => teamOrder(left) - teamOrder(right) || left.localeCompare(right, 'ru'))

  return (
    <div className="ui-modal-backdrop admin-match-details-backdrop" role="presentation" onMouseDown={onClose}>
      <section className="ui-modal admin-match-details-modal" role="dialog" aria-modal="true" aria-labelledby="admin-match-details-title" onMouseDown={(event) => event.stopPropagation()}>
        <div className="ui-modal-header">
          <div>
            <span className="ui-section-title">Матч</span>
            <h2 id="admin-match-details-title">{match.map}</h2>
          </div>
          <button className="ui-modal-close" type="button" aria-label="Закрыть детали матча" onClick={onClose}>×</button>
        </div>
        <div className="admin-match-details-meta">
          <span>{dateFormatter.format(new Date(match.startedAt))}</span>
          <span>{formatDuration(match.durationMs)}</span>
          <span>Участников: {match.playerCount}</span>
        </div>
        <div className="admin-match-team-list">
          {teams.map(([team, players]) => {
            const winner = team.toLowerCase() === match.winningTeam?.toLowerCase()
            return (
              <section key={team} className={`admin-match-team ${teamClass(team)} ${winner ? 'is-winner' : ''}`}>
                <header className="admin-match-team-header">
                  <h3>{teamLabel(team)}</h3>
                  {winner ? <span className="ui-badge ui-badge-success">Победа</span> : null}
                </header>
                <div className="admin-match-team-table-wrap">
                  <table className="admin-match-team-table">
                    <thead><tr><th>Никнейм</th><th>K / D / A</th><th>Урон</th><th>Техника</th><th>K/D</th></tr></thead>
                    <tbody>{players.map((player) => (
                      <tr key={player.playerId} className={player.playerId === playerId ? 'is-profile-player' : ''}>
                        <td><span className="admin-match-player"><PlayerHead playerId={player.playerId} nickname={player.nickname} /><strong>{player.nickname}</strong></span></td>
                        <td>{player.kills} / {player.deaths} / {player.assists}</td>
                        <td>{numberFormatter.format(player.damageDealt)}</td>
                        <td>{player.vehicleDestructions}</td>
                        <td>{player.kd}</td>
                      </tr>
                    ))}</tbody>
                  </table>
                </div>
              </section>
            )
          })}
        </div>
      </section>
    </div>
  )
}

export default function AdminUserMatchesModal({ userId, username, onClose }: { userId: string; username: string; onClose: () => void }) {
  const [offset, setOffset] = useState(0)
  const [data, setData] = useState<PlayerMatchesResponse | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [details, setDetails] = useState<MatchResponse | null>(null)
  const [detailsLoading, setDetailsLoading] = useState<string | null>(null)

  useEffect(() => {
    const previousBodyOverflow = document.body.style.overflow
    const previousHtmlOverflow = document.documentElement.style.overflow

    document.body.style.overflow = 'hidden'
    document.documentElement.style.overflow = 'hidden'

    return () => {
      document.body.style.overflow = previousBodyOverflow
      document.documentElement.style.overflow = previousHtmlOverflow
    }
  }, [])

  useEffect(() => {
    let cancelled = false
    setLoading(true)
    getPlayerMatches(userId, { limit: 10, offset })
      .then((matches) => {
        if (!cancelled) {
          setData(matches)
          setError(null)
        }
      })
      .catch((reason) => {
        if (cancelled) return
        if (reason instanceof ApiError && reason.status === 404) {
          setData(null)
          setError(null)
          return
        }
        setError(toDisplayError(reason, 'Не удалось загрузить матчи.'))
      })
      .finally(() => { if (!cancelled) setLoading(false) })
    return () => { cancelled = true }
  }, [offset, userId])

  const openDetails = async (gameId: string) => {
    setDetailsLoading(gameId)
    try {
      setDetails(await getMatch(gameId))
    } catch (reason) {
      setError(toDisplayError(reason, 'Не удалось открыть детали матча.'))
    } finally {
      setDetailsLoading(null)
    }
  }

  const total = data?.pagination.total ?? 0

  return (
    <AppPortal>
      <div className="ui-modal-backdrop admin-user-matches-backdrop" role="presentation" onMouseDown={onClose}>
        <section className="ui-modal admin-user-matches-modal" role="dialog" aria-modal="true" aria-labelledby="admin-user-matches-title" onMouseDown={(event) => event.stopPropagation()}>
          <div className="ui-modal-header">
            <div>
              <h2 id="admin-user-matches-title" className="ui-modal-title">Матчи: {username}</h2>
              <p className="admin-inline-muted">Последние сыгранные матчи и их результаты.</p>
            </div>
            <button className="ui-modal-close" type="button" aria-label="Закрыть матчи" onClick={onClose}>×</button>
          </div>
          <div className="admin-user-matches-body">
            {error ? <div className="ui-alert ui-alert-error">{error}</div> : null}
            {loading && !data ? <div className="admin-user-matches-loading">Загружаем матчи…</div> : null}
            {!loading && !error && !data ? <div className="admin-user-matches-empty">Матчей пока нет.</div> : null}
            {data?.matches.length ? <div className="admin-user-match-list">{data.matches.map((match) => {
              const result = outcome(match.teamWon)
              return (
                <button key={match.gameId} type="button" className="admin-user-match-row" onClick={() => void openDetails(match.gameId)}>
                  <span className={`admin-user-match-result ${result.className}`}>{result.label}</span>
                  <span className="admin-user-match-main"><strong>{match.map}</strong><small>{dateFormatter.format(new Date(match.startedAt))}</small></span>
                  <span><small>Длительность</small><strong>{formatDuration(match.durationMs)}</strong></span>
                  <span><small>Победитель</small><strong>{teamLabel(match.winningTeam)}</strong></span>
                  <span className="admin-user-match-open">{detailsLoading === match.gameId ? 'Загрузка…' : 'Подробнее →'}</span>
                </button>
              )
            })}</div> : null}
            {total > 10 ? <div className="admin-user-matches-pagination">
              <button className="btn btn-sm" type="button" disabled={offset === 0 || loading} onClick={() => setOffset(Math.max(0, offset - 10))}>Назад</button>
              <span>{offset + 1}–{Math.min(offset + 10, total)} из {total}</span>
              <button className="btn btn-sm" type="button" disabled={offset + 10 >= total || loading} onClick={() => setOffset(offset + 10)}>Дальше</button>
            </div> : null}
          </div>
        </section>
      </div>
      {details ? <MatchDetailsModal match={details} playerId={userId} onClose={() => setDetails(null)} /> : null}
    </AppPortal>
  )
}
