export default function ProfileMatchSummarySkeleton() {
  return (
    <div className="profile-match-summary profile-match-summary--loading" aria-label="Загрузка статистики">
      {Array.from({ length: 6 }, (_, index) => (
        <div key={index}>
          <span className="profile-shimmer profile-shimmer-stat-label" />
          <span className="profile-shimmer profile-shimmer-stat-value" />
        </div>
      ))}
    </div>
  )
}
