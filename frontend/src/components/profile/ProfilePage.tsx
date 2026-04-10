import { useEffect, useState } from 'react'
import { Toaster } from 'react-hot-toast'
import { useQuery } from '../../util/query'
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

export default function ProfilePage() {
  const query = useQuery()
  const { authToken, status, data, error, setData, setAuthUser, reload, reloadSquads, logout } = useProfileDashboard(query)

  const [activeTab, setActiveTab] = useState<ProfileTab>(() => normalizeRequestedTab(query.get('tab')))
  const [nicknameDraft, setNicknameDraft] = useState('')
  const [isUpdatingNickname, setIsUpdatingNickname] = useState(false)
  const [skinVersion, setSkinVersion] = useState(() => Date.now())
  const [skinFailed, setSkinFailed] = useState(false)
  const [showSkeletonOverlay, setShowSkeletonOverlay] = useState(true)
  const [contentVisible, setContentVisible] = useState(false)

  const handleTabChange = (tab: ProfileTab) => {
    setActiveTab(tab)

    if (typeof window === 'undefined') return

    const url = new URL(window.location.href)
    url.searchParams.set('tab', tab)
    window.history.replaceState(null, '', url.pathname + url.search)
  }

  useEffect(() => {
    setActiveTab(normalizeRequestedTab(query.get('tab')))
  }, [query])

  useEffect(() => {
    if (data) {
      setNicknameDraft(data.user.username)
      setSkinFailed(false)
    }
  }, [data])

  if (status === 'unauthorized') {
    return <ProfileUnauthorizedState />
  }

  if (status === 'error' || (!data && status !== 'loading')) {
    return <ProfileErrorState error={error} onRetry={() => void reload()} onLogout={logout} />
  }

  const isLoading = status === 'loading'

  useEffect(() => {
    if (isLoading) {
      setShowSkeletonOverlay(true)
      setContentVisible(false)
      return
    }

    if (!data) {
      setShowSkeletonOverlay(false)
      setContentVisible(false)
      return
    }

    setShowSkeletonOverlay(true)
    const frameId = window.requestAnimationFrame(() => setContentVisible(true))
    const timeoutId = window.setTimeout(() => setShowSkeletonOverlay(false), 280)

    return () => {
      window.cancelAnimationFrame(frameId)
      window.clearTimeout(timeoutId)
    }
  }, [data, isLoading, activeTab])

  return (
    <div className="ui-kit-page profile-page">
      <Toaster
        position="top-right"
        containerStyle={{
          zIndex: 1000,
        }}
        toastOptions={{
          duration: 3000,
          style: {
            background: '#19191c',
            color: '#fff',
            border: '1px solid rgba(255,255,255,.08)',
            borderRadius: '0',
          },
        }}
      />
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
            <div key={activeTab} className={`profile-content-stage ${contentVisible ? 'is-visible' : ''}`}>
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
                  data={data}
                  authToken={authToken}
                  nicknameDraft={nicknameDraft}
                  setNicknameDraft={setNicknameDraft}
                  isUpdatingNickname={isUpdatingNickname}
                  setIsUpdatingNickname={setIsUpdatingNickname}
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

          {showSkeletonOverlay ? (
            <div className={`profile-loading-overlay ${!isLoading && contentVisible ? 'is-exiting' : ''}`} aria-hidden>
              <ProfileSkeleton activeTab={activeTab} />
            </div>
          ) : null}
        </div>
      </div>
    </div>
  )
}
