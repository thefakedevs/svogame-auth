import { useEffect, useMemo, useState } from 'react'
import { toDisplayError } from '../../api/http'
import { getMyReferralStats, type ReferralStatsResponse } from '../../api/referrals'
import ErrorState from '../ErrorState'
import LoadingState from '../LoadingState'

function buildStatsRange() {
  const now = new Date()
  const to = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate(), 23, 59, 59))
  const from = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate() - 6, 0, 0, 0))
  return {
    from: from.toISOString(),
    to: to.toISOString(),
  }
}

function DailyReferralChart({ stats }: { stats: ReferralStatsResponse }) {
  const maxValue = Math.max(1, ...stats.buckets.map((bucket) => bucket.registrations))

  return (
    <div className="profile-referral-chart" aria-label="Дневная статистика реферальных регистраций">
      {stats.buckets.map((bucket) => (
        <div key={bucket.date} className="profile-referral-chart__bar-wrap">
          <span className="profile-referral-chart__value">{bucket.registrations}</span>
          <span
            className="profile-referral-chart__bar"
            style={{ height: `${Math.max(8, (bucket.registrations / maxValue) * 100)}%` }}
            aria-hidden
          />
          <span className="profile-referral-chart__date">{bucket.date.slice(5)}</span>
        </div>
      ))}
    </div>
  )
}

export default function ProfileReferralsTab({ authToken }: { authToken: string | null }) {
  const [stats, setStats] = useState<ReferralStatsResponse | null>(null)
  const [error, setError] = useState('')
  const range = useMemo(() => buildStatsRange(), [])

  useEffect(() => {
    if (!authToken) return

    let cancelled = false
    setStats(null)
    setError('')

    const run = async () => {
      try {
        const response = await getMyReferralStats(authToken, range)
        if (!cancelled) setStats(response)
      } catch (cause) {
        if (!cancelled) setError(toDisplayError(cause, 'Не удалось загрузить реферальную статистику.'))
      }
    }

    void run()
    return () => {
      cancelled = true
    }
  }, [authToken, range])

  return (
    <div className="profile-main">
      <section className="card profile-panel">
        <div className="ui-card-header">
          <h2 className="card-title">Реферальная статистика</h2>
          <span className="ui-badge ui-badge-neutral">UTC</span>
        </div>
        {error ? <ErrorState message={error} /> : null}
        {!stats && !error ? <LoadingState title="Загружаем статистику" /> : null}
        {stats ? (
          <div className="profile-stack">
            <div className="profile-stats">
              <article className="card profile-stat-card">
                <span className="profile-stat-label">Регистрации</span>
                <strong className="profile-stat-value">{stats.total}</strong>
                <span className="profile-stat-note">За последние 7 дней</span>
              </article>
              <article className="card profile-stat-card">
                <span className="profile-stat-label">Дней в графике</span>
                <strong className="profile-stat-value">{stats.buckets.length}</strong>
                <span className="profile-stat-note">Backend возвращает дни с нулем</span>
              </article>
            </div>
            <DailyReferralChart stats={stats} />
          </div>
        ) : null}
      </section>
    </div>
  )
}
