import { useEffect, useState } from 'react'
import { toDisplayError } from '../../../api/http'
import { searchUsers, type UserSearchItemResponse } from '../../../api/squads'

export function useSquadUserSearch(authToken: string | null, query: string) {
  const [items, setItems] = useState<UserSearchItemResponse[]>([])
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState('')

  useEffect(() => {
    if (!authToken) {
      setItems([])
      setError('')
      return
    }

    const trimmed = query.trim()
    if (trimmed.length < 3) {
      setItems([])
      setError('')
      return
    }

    let cancelled = false
    const timerId = window.setTimeout(async () => {
      try {
        setIsLoading(true)
        setError('')
        const result = await searchUsers(authToken, trimmed, 10)
        if (cancelled) return
        setItems(result)
      } catch (cause) {
        if (cancelled) return
        setItems([])
        setError(toDisplayError(cause, 'Не удалось найти игроков.'))
      } finally {
        if (!cancelled) {
          setIsLoading(false)
        }
      }
    }, 250)

    return () => {
      cancelled = true
      window.clearTimeout(timerId)
    }
  }, [authToken, query])

  return { items, isLoading, error }
}
