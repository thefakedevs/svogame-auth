import { useMemo } from 'react'
import { useSearch } from './history'

export function useQueryParams() {
  const search = useSearch()

  return useMemo(() => new URLSearchParams(search), [search])
}
