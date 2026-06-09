import type { ReferralSource } from '../../api/referrals'

const REFERRAL_STORAGE_KEY = 'auth-referral'
const REFERRAL_CODE_PATTERN = /^[A-Za-z0-9_-]{3,32}$/

export interface StoredReferralCode {
  code: string
  source: ReferralSource
}

export function normalizeReferralCode(value: string): string {
  return value.trim().toUpperCase()
}

export function isReferralCodeFormatValid(value: string): boolean {
  return REFERRAL_CODE_PATTERN.test(value)
}

export function loadStoredReferralCode(): StoredReferralCode | null {
  if (typeof window === 'undefined') return null

  try {
    const raw = window.sessionStorage.getItem(REFERRAL_STORAGE_KEY)
    if (!raw) return null

    const data = JSON.parse(raw) as Partial<StoredReferralCode>
    if (typeof data.code !== 'string' || (data.source !== 'link' && data.source !== 'manual')) {
      return null
    }

    const code = normalizeReferralCode(data.code)
    if (!isReferralCodeFormatValid(code)) return null

    return {
      code,
      source: data.source,
    }
  } catch {
    return null
  }
}

export function saveStoredReferralCode(input: StoredReferralCode | null) {
  if (typeof window === 'undefined') return

  if (!input) {
    window.sessionStorage.removeItem(REFERRAL_STORAGE_KEY)
    return
  }

  window.sessionStorage.setItem(
    REFERRAL_STORAGE_KEY,
    JSON.stringify({
      code: normalizeReferralCode(input.code),
      source: input.source,
    }),
  )
}
