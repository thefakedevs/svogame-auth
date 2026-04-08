import { useEffect, useState } from 'react'
import { Toaster } from 'react-hot-toast'
import { useQuery } from '../../util/query'
import ProfileOverviewTab from './ProfileOverviewTab'
import ProfileSettingsTab from './ProfileSettingsTab'
import ProfileSkeleton from './ProfileSkeleton'
import ProfileSquadsTab from './ProfileSquadsTab'
import { ProfileErrorState, ProfileUnauthorizedState } from './ProfileStates'
import { useProfileDashboard } from './useProfileDashboard'
import type { ProfileTab } from './types'
import '../../pages/ProfilePage.css'

function normalizeRequestedTab(value: string | null): ProfileTab {
  if (value === 'squads') return 'squads'
  if (value === 'settings') return 'settings'
  return 'overview'
}

export default function ProfilePage() {
  const query = useQuery()
  const {
    authToken,
    status,
    data,
    error,
    setData,
    setAuthUser,
    reload,
    logout,
  } = useProfileDashboard(query)

  const [activeTab, setActiveTab] = useState<ProfileTab>(() => normalizeRequestedTab(query.get('tab')))
  const [nicknameDraft, setNicknameDraft] = useState('')
  const [isUpdatingNickname, setIsUpdatingNickname] = useState(false)
  const [skinVersion, setSkinVersion] = useState(() => Date.now())
  const [skinFailed, setSkinFailed] = useState(false)
  const [showSkeletonOverlay, setShowSkeletonOverlay] = useState(true)
  const [contentVisible, setContentVisible] = useState(false)

  useEffect(() => {
    setActiveTab(normalizeRequestedTab(query.get('tab')))
  }, [query])

  useEffect(() => {
    if (data) {
      setNicknameDraft(data.user.username)
      setSkinFailed(false)
    }
  }, [data])

  useEffect(() => {
    if (status === 'loading') {
      setShowSkeletonOverlay(true)
      if (!data) setContentVisible(false)
      return
    }

    if (status === 'loaded' && data) {
      setShowSkeletonOverlay(true)
      const frameId = window.requestAnimationFrame(() => setContentVisible(true))
      const timeoutId = window.setTimeout(() => setShowSkeletonOverlay(false), 320)
      return () => {
        window.cancelAnimationFrame(frameId)
        window.clearTimeout(timeoutId)
      }
    }

    setShowSkeletonOverlay(false)
    setContentVisible(false)
  }, [data, status])

  if (status === 'unauthorized') {
    return <ProfileUnauthorizedState />
  }

  if (status === 'error' || (!data && status !== 'loading')) {
    return <ProfileErrorState error={error} onRetry={() => void reload()} onLogout={logout} />
  }

  return (
    <div className="ui-kit-page profile-page">
      <Toaster
        position="top-right"
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
        <div className="profile-transition-shell">
          {data ? (
            <div className={`profile-content-stage ${contentVisible ? 'is-visible' : ''}`}>
              <div className="profile-topbar">
                <nav className="profile-tabs" aria-label="Разделы профиля">
                  {([
                    ['overview', 'Обзор'],
                    ['squads', 'Сквад'],
                    ['settings', 'Настройки'],
                  ] as const).map(([key, label]) => (
                    <button
                      key={key}
                      className={`profile-tab ${activeTab === key ? 'is-active' : ''}`}
                      type="button"
                      onClick={() => setActiveTab(key)}
                    >
                      <span>{label}</span>
                    </button>
                  ))}
                </nav>
              </div>

              {activeTab === 'overview' ? (
                <ProfileOverviewTab
                  data={data}
                  skinFailed={skinFailed}
                  setSkinFailed={setSkinFailed}
                  setActiveTab={setActiveTab}
                  skinVersion={skinVersion}
                />
              ) : null}

              {activeTab === 'squads' ? (
                <ProfileSquadsTab
                  data={data}
                  authToken={authToken}
                  onChanged={reload}
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
            <div className={`profile-loading-overlay ${status === 'loaded' && contentVisible ? 'is-exiting' : ''}`} aria-hidden>
              <ProfileSkeleton />
            </div>
          ) : null}
        </div>
      </div>
    </div>
  )
}
