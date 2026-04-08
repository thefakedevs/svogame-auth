export default function ProfileSkeleton() {
  return (
    <>
      <div className="profile-tabs profile-tabs--skeleton">
        <div className="profile-shimmer profile-shimmer-tab profile-shimmer-tab--overview" />
        <div className="profile-shimmer profile-shimmer-tab profile-shimmer-tab--squads" />
        <div className="profile-shimmer profile-shimmer-tab profile-shimmer-tab--settings" />
      </div>
      <div className="profile-skeleton-grid">
        <div className="card profile-skeleton-identity">
          <div className="profile-skeleton-head">
            <div className="profile-shimmer profile-shimmer-avatar" />
            <div className="profile-skeleton-head-copy">
              <div className="profile-shimmer profile-shimmer-name" />
              <div className="profile-shimmer profile-shimmer-badges" />
            </div>
          </div>
          <div className="profile-skeleton-toolbar">
            <div className="profile-shimmer profile-shimmer-kicker" />
            <div className="profile-shimmer profile-shimmer-button" />
          </div>
          <div className="profile-shimmer profile-shimmer-skin" />
          <div className="profile-skeleton-meta">
            <div className="profile-shimmer profile-shimmer-meta" />
            <div className="profile-shimmer profile-shimmer-meta" />
            <div className="profile-shimmer profile-shimmer-meta" />
          </div>
        </div>
        <div className="profile-skeleton-main">
          <div className="profile-skeleton-stats">
            {Array.from({ length: 3 }, (_, index) => (
              <div key={index} className="card profile-skeleton-stat-card">
                <div className="profile-shimmer profile-shimmer-stat-label" />
                <div className="profile-shimmer profile-shimmer-stat-value" />
                <div className="profile-shimmer profile-shimmer-stat-note" />
              </div>
            ))}
          </div>
          <div className="card profile-skeleton-panel">
            <div className="profile-skeleton-panel-head">
              <div className="profile-shimmer profile-shimmer-section" />
              <div className="profile-shimmer profile-shimmer-button profile-shimmer-button--sm" />
            </div>
            <div className="profile-shimmer profile-shimmer-item" />
            <div className="profile-shimmer profile-shimmer-item" />
            <div className="profile-shimmer profile-shimmer-item" />
          </div>
          <div className="card profile-skeleton-panel profile-skeleton-panel--summary">
            <div className="profile-skeleton-panel-head">
              <div className="profile-shimmer profile-shimmer-section" />
              <div className="profile-shimmer profile-shimmer-button profile-shimmer-button--sm" />
            </div>
            <div className="profile-shimmer profile-shimmer-summary-line" />
            <div className="profile-shimmer profile-shimmer-summary-line profile-shimmer-summary-line--short" />
            <div className="profile-skeleton-chip-row">
              <div className="profile-shimmer profile-shimmer-chip" />
              <div className="profile-shimmer profile-shimmer-chip" />
              <div className="profile-shimmer profile-shimmer-chip" />
            </div>
          </div>
        </div>
      </div>
    </>
  )
}
