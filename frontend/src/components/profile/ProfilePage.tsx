import { useEffect, useState } from 'react'
import { getMyReferralCampaigns } from '../../api/referrals'
import { replaceUrl } from '../../util/navigation'
import { useQuery } from '../../util/query'
import { consumeNewPlayerOnboardingPending } from '../../shared/session/new-player-onboarding'
import {
  loadCachedReferralCampaignAccess,
  saveCachedReferralCampaignAccess,
} from '../../shared/session/referral-campaign-cache'
import '../../pages/ProfilePage.css'
import ProfileNewPlayerModal from './ProfileNewPlayerModal'
import ProfileMatchesTab from './ProfileMatchesTab'
import ProfileOverviewTab from './ProfileOverviewTab'
import ProfileReferralsTab from './ProfileReferralsTab'
import ProfileSettingsTab from './ProfileSettingsTab'
import ProfileSkeleton from './ProfileSkeleton'
import { ProfileErrorState, ProfileUnauthorizedState } from './ProfileStates'
import ProfileSquadsTab from './ProfileSquadsTab'
import type { ProfileTab } from './types'
import { useProfileDashboard } from './useProfileDashboard'

function normalizeRequestedTab(value: string | null): ProfileTab {
  if (value === 'matches') return 'matches'
  if (value === 'squads') return 'squads'
  if (value === 'referrals') return 'referrals'
  if (value === 'settings') return 'settings'
  return 'overview'
}

export default function ProfilePage({ defaultTab }: { defaultTab?: ProfileTab }) {
  const query = useQuery()
  const { authToken, status, data, error, setData, setAuthUser, reload, reloadSquads, logout } = useProfileDashboard(query)
  const [skinVersion, setSkinVersion] = useState(() => Date.now())
  const [skinFailed, setSkinFailed] = useState(false)
  const [showNewPlayerModal, setShowNewPlayerModal] = useState(false)
  const [checkedReferralCampaignAccess, setCheckedReferralCampaignAccess] = useState<boolean | null>(null)
  const cachedReferralCampaignAccess = data ? loadCachedReferralCampaignAccess(data.user.id) : false
  const hasReferralCampaigns = checkedReferralCampaignAccess ?? cachedReferralCampaignAccess

  const requestedTab = normalizeRequestedTab(query.get('tab') ?? defaultTab ?? null)
  const activeTab = requestedTab === 'referrals' && !hasReferralCampaigns ? 'overview' : requestedTab
  const isLoading = status === 'loading'

  const handleTabChange = (tab: ProfileTab) => {
    if (typeof window === 'undefined') return
    const url = new URL(window.location.href)
    url.searchParams.set('tab', tab)
    replaceUrl(url.pathname + url.search)
  }

  useEffect(() => {
    if (!data || status !== 'loaded') return
    if (consumeNewPlayerOnboardingPending()) {
      setShowNewPlayerModal(true)
    }
  }, [data, status])

  useEffect(() => {
    setCheckedReferralCampaignAccess(null)
  }, [data?.user.id])

  useEffect(() => {
    if (!authToken || status !== 'loaded' || !data) {
      setCheckedReferralCampaignAccess(null)
      return
    }

    let cancelled = false
    const userId = data.user.id

    const run = async () => {
      try {
        const response = await getMyReferralCampaigns(authToken)
        if (cancelled) return

        const nextHasAccess = response.total > 0 || response.items.length > 0
        saveCachedReferralCampaignAccess(userId, nextHasAccess)
        setCheckedReferralCampaignAccess(nextHasAccess)
      } catch {
        if (!cancelled) setCheckedReferralCampaignAccess(null)
      }
    }

    void run()
    return () => {
      cancelled = true
    }
  }, [authToken, data, status])

  if (status === 'unauthorized') return <ProfileUnauthorizedState />
  if (status === 'error' || (!data && status !== 'loading')) {
    return <ProfileErrorState error={error} onRetry={() => void reload()} onLogout={logout} />
  }

  const tabs = [
    ['overview', 'Обзор'],
    ['matches', 'Матчи'],
    ['squads', 'Сквад'],
    ...(hasReferralCampaigns ? ([['referrals', 'Рефералки']] as const) : []),
    ['settings', 'Настройки'],
  ] as const

  return (
    <div className="ui-kit-page profile-page">
      <div className="profile-shell">
        <div className="profile-topbar">
          <nav className="profile-tabs" aria-label="Разделы профиля">
            {tabs.map(([key, label]) => (
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

              {activeTab === 'matches' ? <ProfileMatchesTab playerId={data.user.id} /> : null}

              {activeTab === 'squads' ? (
                <ProfileSquadsTab data={data} authToken={authToken} onChanged={reloadSquads} />
              ) : null}

              {activeTab === 'referrals' ? (
                <ProfileReferralsTab authToken={authToken} />
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
      {showNewPlayerModal ? <ProfileNewPlayerModal onClose={() => setShowNewPlayerModal(false)} /> : null}
    </div>
  )
}
