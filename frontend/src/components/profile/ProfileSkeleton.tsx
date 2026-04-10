import type { ProfileTab } from './types'

function OverviewSkeleton() {
  return (
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
        </div>
      </div>
      <div className="profile-skeleton-main">
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
  )
}

function SquadsSkeleton() {
  return (
    <div className="profile-split">
      <div className="profile-stack">
        <div className="card profile-skeleton-panel">
          <div className="profile-skeleton-panel-head">
            <div className="profile-shimmer profile-shimmer-section" />
            <div className="profile-shimmer profile-shimmer-chip" />
          </div>
          <div className="profile-shimmer profile-shimmer-item" />
          <div className="profile-shimmer profile-shimmer-item" />
          <div className="profile-shimmer profile-shimmer-item" />
          <div className="profile-shimmer profile-shimmer-item" />
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
      </div>
      <div className="card profile-skeleton-panel">
        <div className="profile-skeleton-panel-head">
          <div className="profile-shimmer profile-shimmer-section" />
          <div className="profile-shimmer profile-shimmer-chip" />
        </div>
        {Array.from({ length: 5 }, (_, index) => (
          <div key={index} className="profile-shimmer profile-shimmer-item" />
        ))}
      </div>
    </div>
  )
}

function SettingsSkeleton() {
  return (
    <div className="profile-split">
      <div className="card profile-skeleton-panel">
        <div className="profile-skeleton-panel-head">
          <div className="profile-shimmer profile-shimmer-section" />
          <div className="profile-shimmer profile-shimmer-button profile-shimmer-button--sm" />
        </div>
        <div className="profile-shimmer profile-shimmer-skin" />
        <div className="profile-shimmer profile-shimmer-item" />
        <div className="profile-shimmer profile-shimmer-item" />
      </div>
      <div className="profile-stack">
        <div className="card profile-skeleton-panel">
          <div className="profile-skeleton-panel-head">
            <div className="profile-shimmer profile-shimmer-section" />
          </div>
          <div className="profile-shimmer profile-shimmer-item" />
          <div className="profile-shimmer profile-shimmer-item" />
          <div className="profile-shimmer profile-shimmer-item" />
          <div className="profile-shimmer profile-shimmer-item" />
          <div className="profile-shimmer profile-shimmer-button profile-shimmer-button--sm" />
        </div>
        <div className="card profile-skeleton-panel">
          <div className="profile-skeleton-panel-head">
            <div className="profile-shimmer profile-shimmer-section" />
          </div>
          <div className="profile-shimmer profile-shimmer-item" />
          <div className="profile-shimmer profile-shimmer-item" />
        </div>
      </div>
    </div>
  )
}

export default function ProfileSkeleton({ activeTab = 'overview' }: { activeTab?: ProfileTab }) {
  return (
    <>
      {activeTab === 'squads' ? <SquadsSkeleton /> : null}
      {activeTab === 'settings' ? <SettingsSkeleton /> : null}
      {activeTab === 'overview' ? <OverviewSkeleton /> : null}
    </>
  )
}
