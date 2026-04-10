import { useState } from 'react'
import { useQuery } from '../../util/query'
import { replaceUrl } from '../../util/navigation'
import '../../pages/ProfilePage.css'
import ProfileOverviewTab from './ProfileOverviewTab'
import ProfileSettingsTab from './ProfileSettingsTab'
import ProfileSkeleton from './ProfileSkeleton'
import { ProfileErrorState, ProfileUnauthorizedState } from './ProfileStates'
import ProfileSquadsTab from './ProfileSquadsTab'
import type { ProfileTab } from './types'
import { useProfileDashboard } from './useProfileDashboard'

function normalizeRequestedTab(value: string | null): ProfileTab {
  if (value === 'squads') return 'squads'
  if (value === 'settings') return 'settings'
  return 'overview'
}

export default function ProfilePage({ defaultTab }: { defaultTab?: ProfileTab }) {
  const query = useQuery()
  const { authToken, status, data, error, setData, setAuthUser, reload, reloadSquads, logout } = useProfileDashboard(query)
  const [skinVersion, setSkinVersion] = useState(() => Date.now())
  const [skinFailed, setSkinFailed] = useState(false)

  const activeTab = normalizeRequestedTab(query.get('tab') ?? defaultTab ?? null)
  const isLoading = status === 'loading'

  const handleTabChange = (tab: ProfileTab) => {
    if (typeof window === 'undefined') return

    const url = new URL(window.location.href)
    url.searchParams.set('tab', tab)
    replaceUrl(url.pathname + url.search)
  }

  if (status === 'unauthorized') {
    return <ProfileUnauthorizedState />
  }

  if (status === 'error' || (!data && status !== 'loading')) {
    return <ProfileErrorState error={error} onRetry={() => void reload()} onLogout={logout} />
  }

  return (
    <div className="ui-kit-page profile-page">
      <div className="profile-shell">
        <div className="profile-topbar">
          <nav className="profile-tabs" aria-label="Разделы профиля">
            {([
              ['overview', 'Обзор'],
              ['squads', 'Сквад'],
              ['settings', 'Настройки'],
            ] as const).map(([key, label]) => (
              <button
                key={key}
                className={`profile-tab ${activeTab === key ? 'is-active' : ''} ${key === 'squads' && (data?.squadInvites.length ?? 0) > 0 ? 'has-notification' : ''}`}
                type="button"
                onClick={() => handleTabChange(key)}
              >
                <span>{label}</span>
              </button>
            ))}
          </nav>
        </div>

        <div className="profile-transition-shell">
          {!isLoading && data ? (
            <div key={activeTab} className="profile-content-stage is-visible">
              {activeTab === 'overview' ? (
                <ProfileOverviewTab
                  data={data}
                  skinFailed={skinFailed}
                  setSkinFailed={setSkinFailed}
                  setActiveTab={handleTabChange}
                  skinVersion={skinVersion}
                />
              ) : null}

              {activeTab === 'squads' ? (
                <ProfileSquadsTab
                  data={data}
                  authToken={authToken}
                  onChanged={reloadSquads}
                />
              ) : null}

              {activeTab === 'settings' ? (
                <ProfileSettingsTab
                  key={data.user.id}
                  data={data}
                  authToken={authToken}
                  setData={setData}
                  setAuthUser={setAuthUser}
                  onLogout={logout}
                  skinFailed={skinFailed}
                  setSkinFailed={setSkinFailed}
                  skinVersion={skinVersion}
                  setSkinVersion={setSkinVersion}
                />
              ) : null}
            </div>
          ) : null}

          {isLoading ? (
            <div className="profile-loading-overlay" aria-hidden>
              <ProfileSkeleton activeTab={activeTab} />
            </div>
          ) : null}
        </div>
      </div>
    </div>
  )
}
