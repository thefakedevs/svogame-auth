const REFERRAL_CAMPAIGN_CACHE_PREFIX = 'profile-referral-campaigns'

function cacheKey(userId: string) {
  return `${REFERRAL_CAMPAIGN_CACHE_PREFIX}:${userId}`
}

export function loadCachedReferralCampaignAccess(userId: string): boolean {
  if (typeof window === 'undefined') return false

  try {
    return window.localStorage.getItem(cacheKey(userId)) === '1'
  } catch {
    return false
  }
}

export function saveCachedReferralCampaignAccess(userId: string, hasAccess: boolean): void {
  if (typeof window === 'undefined') return

  try {
    const key = cacheKey(userId)
    const nextValue = hasAccess ? '1' : '0'
    if (window.localStorage.getItem(key) !== nextValue) {
      window.localStorage.setItem(key, nextValue)
    }
  } catch {
    // localStorage can be blocked; the API check still owns the live UI state.
  }
}
