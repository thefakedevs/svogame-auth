import type { SkinRarity } from '../api/inventory'

/** Порядок редкости для сортировки и сравнения (меньше = ниже редкость). */
export const SKIN_RARITY_RANK: Record<SkinRarity, number> = {
  common: 0,
  rare: 1,
  legendary: 2,
}

export function skinRarityRank(value: SkinRarity | null | undefined): number {
  if (!value) return 999
  return SKIN_RARITY_RANK[value] ?? 500
}

export function uniqueSortedWeaponKeys(keys: (string | null | undefined)[]): string[] {
  const set = new Set<string>()
  for (const raw of keys) {
    const key = raw?.trim().replace("taczgun:", '')
    if (key) set.add(key)
  }
  return [...set].sort((a, b) => a.localeCompare(b, 'ru'))
}
