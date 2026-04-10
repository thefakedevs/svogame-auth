import type { PowData } from './token'

const POW_STORAGE_KEY = 'pow-data'

export function loadStoredPowData(): PowData | null {
  if (typeof window === 'undefined') {
    return null
  }

  try {
    const rawValue = window.sessionStorage.getItem(POW_STORAGE_KEY)
    if (!rawValue) {
      return null
    }

    const parsed = JSON.parse(rawValue) as Partial<PowData>
    if (!parsed.solution || !parsed.prefix) {
      return null
    }

    return {
      solution: parsed.solution,
      prefix: parsed.prefix,
    }
  } catch {
    return null
  }
}

export function saveStoredPowData(data: PowData | null) {
  if (typeof window === 'undefined') {
    return
  }

  if (!data) {
    window.sessionStorage.removeItem(POW_STORAGE_KEY)
    return
  }

  window.sessionStorage.setItem(POW_STORAGE_KEY, JSON.stringify(data))
}
