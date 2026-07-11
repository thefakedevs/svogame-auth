import type { PlayerStatsResponse } from '../../api/metrics'

const numberFormatter = new Intl.NumberFormat('ru-RU', { maximumFractionDigits: 2 })

export default function ProfileMatchSummary({ stats }: { stats: PlayerStatsResponse }) {
  const values = [
    ['Матчи', stats.matchesPlayed],
    ['Победы', `${stats.winRate}%`],
    ['Убийства', stats.kills],
    ['Смерти', stats.deaths],
    ['K/D', stats.kd],
    ['Урон', numberFormatter.format(stats.damageDealt)],
  ]

  return (
    <div className="profile-match-summary">
      {values.map(([label, value]) => (
        <div key={label}>
          <span>{label}</span>
          <strong>{value}</strong>
        </div>
      ))}
    </div>
  )
}
