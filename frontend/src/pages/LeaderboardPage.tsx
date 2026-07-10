import { useEffect, useState } from 'react'
import { getLeaderboard, type LeaderboardMetric, type LeaderboardResponse, type MetricsPeriod } from '../api/metrics'
import { toDisplayError } from '../api/http'
import PlayerHead from '../components/metrics/PlayerHead'
import './LeaderboardPage.css'

const metricOptions: Array<[LeaderboardMetric, string]> = [
  ['kills', 'Убийства'],
  ['deaths', 'Смерти'],
  ['damage_dealt', 'Урон'],
  ['kd', 'K/D'],
]

const periods: Array<[MetricsPeriod, string]> = [
  ['day', '24 часа'],
  ['week', '7 дней'],
  ['month', '30 дней'],
  ['all', 'Всё время'],
]

const numberFormatter = new Intl.NumberFormat('ru-RU', { maximumFractionDigits: 2 })

export default function LeaderboardPage() {
  const [metric, setMetric] = useState<LeaderboardMetric>('kills')
  const [period, setPeriod] = useState<MetricsPeriod>('all')
  const [offset, setOffset] = useState(0)
  const [state, setState] = useState<{ data: LeaderboardResponse | null; loading: boolean; error: string | null }>({
    data: null,
    loading: true,
    error: null,
  })

  useEffect(() => {
    let cancelled = false
    setState((current) => ({ ...current, loading: true, error: null }))
    getLeaderboard({ metric, period, limit: 25, offset, minMatches: 1 })
      .then((data) => {
        if (!cancelled) setState({ data, loading: false, error: null })
      })
      .catch((error) => {
        if (!cancelled) setState((current) => ({ ...current, loading: false, error: toDisplayError(error, 'Не удалось загрузить рейтинг.') }))
      })
    return () => { cancelled = true }
  }, [metric, offset, period])

  const selectMetric = (value: LeaderboardMetric) => {
    setMetric(value)
    setOffset(0)
  }
  const selectPeriod = (value: MetricsPeriod) => {
    setPeriod(value)
    setOffset(0)
  }
  const entries = state.data?.entries ?? []
  const total = state.data?.pagination.total ?? 0

  return (
    <main className="leaderboard-page page">
      <section className="card leaderboard-header">
        <div>
          <h1 className="card-title">Таблица лидеров</h1>
          <p className="card-text">Результаты матчей сообщества.</p>
        </div>
        <span className="ui-badge ui-badge-neutral">{numberFormatter.format(total)} участников</span>
      </section>

      <section className="card leaderboard-card">
        <div className="leaderboard-controls">
          <label className="ui-field">
            <span className="ui-label">Показатель</span>
            <span className="leaderboard-select-wrap">
              <select className="ui-input" value={metric} onChange={(event) => selectMetric(event.target.value as LeaderboardMetric)}>
                {metricOptions.map(([value, label]) => <option key={value} value={value}>{label}</option>)}
              </select>
            </span>
          </label>
          <div className="leaderboard-periods" aria-label="Период рейтинга">
            {periods.map(([value, label]) => (
              <button key={value} type="button" className={`btn btn-sm ${period === value ? 'primary' : ''}`} onClick={() => selectPeriod(value)}>
                {label}
              </button>
            ))}
          </div>
        </div>

        {state.error ? <div className="ui-alert ui-alert-error">{state.error}</div> : null}
        <div className={`leaderboard-table-wrap ${state.loading ? 'is-loading' : ''}`} aria-busy={state.loading}>
          <table className="leaderboard-table">
            <thead>
              <tr>
                <th>Место</th><th>Участник</th><th>Матчи</th>
                <th className={metric === 'kills' ? 'is-selected-metric' : ''}>Убийства</th>
                <th className={metric === 'deaths' ? 'is-selected-metric' : ''}>Смерти</th>
                <th className={metric === 'damage_dealt' ? 'is-selected-metric' : ''}>Урон</th>
                <th className={metric === 'kd' ? 'is-selected-metric' : ''}>K/D</th>
              </tr>
            </thead>
            <tbody>
              {entries.map((entry) => (
                <tr key={entry.playerId}>
                  <td><span className={`leaderboard-rank leaderboard-rank--${entry.rank}`}>#{entry.rank}</span></td>
                  <td>
                    <div className="leaderboard-player">
                      <PlayerHead playerId={entry.playerId} nickname={entry.nickname} />
                      <span><strong>{entry.nickname}</strong><small>{entry.playerId}</small></span>
                    </div>
                  </td>
                  <td>{numberFormatter.format(entry.matchesPlayed)}</td>
                  <td className={metric === 'kills' ? 'leaderboard-primary-value' : ''}>{entry.kills}</td>
                  <td className={metric === 'deaths' ? 'leaderboard-primary-value' : ''}>{entry.deaths}</td>
                  <td className={metric === 'damage_dealt' ? 'leaderboard-primary-value' : ''}>{numberFormatter.format(entry.damageDealt)}</td>
                  <td className={metric === 'kd' ? 'leaderboard-primary-value' : ''}>{numberFormatter.format(entry.kd)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {!state.loading && !state.error && entries.length === 0 ? <div className="leaderboard-empty">За выбранный период матчей пока нет.</div> : null}
          {state.loading ? <div className="leaderboard-loading"><span className="ui-spinner" /> Загружаем рейтинг…</div> : null}
        </div>

        {total > 25 ? (
          <div className="leaderboard-pagination">
            <button className="btn btn-sm" type="button" disabled={offset === 0 || state.loading} onClick={() => setOffset(Math.max(0, offset - 25))}>Назад</button>
            <span>{offset + 1}–{Math.min(offset + 25, total)} из {total}</span>
            <button className="btn btn-sm" type="button" disabled={offset + 25 >= total || state.loading} onClick={() => setOffset(offset + 25)}>Дальше</button>
          </div>
        ) : null}
      </section>
    </main>
  )
}
