const NEW_PLAYER_ONBOARDING_KEY = 'new-player-onboarding'

export function markNewPlayerOnboardingPending() {
  if (typeof window === 'undefined') return
  window.sessionStorage.setItem(NEW_PLAYER_ONBOARDING_KEY, '1')
}

export function consumeNewPlayerOnboardingPending() {
  if (typeof window === 'undefined') return false

  const pending = window.sessionStorage.getItem(NEW_PLAYER_ONBOARDING_KEY) === '1'
  if (pending) {
    window.sessionStorage.removeItem(NEW_PLAYER_ONBOARDING_KEY)
  }

  return pending
}
