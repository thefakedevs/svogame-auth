import { useCallback, useEffect, useRef, useState } from 'react'
import { ApiError, toDisplayError } from '../../api/http'
import { getCurrentAppPath, redirectToAuth } from '../../features/auth/lib/navigation'
import { fetchProfileDashboard, fetchSquadDashboardSlice, toAuthUser } from '../../features/profile/lib/dashboard'
import { navigateTo, replaceUrl } from '../../shared/navigation/history'
import { clearAuthSession, setAuthToken, setAuthUser } from '../../shared/session/auth-session'
import { validateTokenFormat } from '../../shared/session/token'
import { useAuthStore } from '../../store/authStore'
import type { ProfileDashboardData, ProfileStatus } from './types'

function isUnauthorizedError(cause: unknown) {
  return cause instanceof ApiError && cause.isAuthError()
}

export function useProfileDashboard(query: URLSearchParams) {
  const authHydrated = useAuthStore((store) => store.hydrated)
  const authToken = useAuthStore((store) => store.token)
  const processedUrlTokenRef = useRef<string | null>(null)

  const [status, setStatus] = useState<ProfileStatus>('loading')
  const [data, setData] = useState<ProfileDashboardData | null>(null)
  const [error, setError] = useState('')

  const markUnauthorized = useCallback(() => {
    clearAuthSession()
    setStatus('unauthorized')
  }, [])

  const loadDashboard = useCallback(async (token: string) => {
    try {
      const nextData = await fetchProfileDashboard(token)
      setAuthUser(toAuthUser(nextData.user))
      setError('')
      setData(nextData)
      setStatus('loaded')
    } catch (cause) {
      if (isUnauthorizedError(cause)) {
        markUnauthorized()
        return
      }

      setError(toDisplayError(cause, 'Не удалось загрузить профиль.'))
      setStatus('error')
    }
  }, [markUnauthorized])

  const reload = useCallback(async () => {
    if (!authToken) {
      setStatus('unauthorized')
      return
    }

    setStatus('loading')
    await loadDashboard(authToken)
  }, [authToken, loadDashboard])

  const reloadSquads = useCallback(async () => {
    if (!authToken) {
      setStatus('unauthorized')
      return
    }

    if (!data) {
      await reload()
      return
    }

    try {
      const nextSlice = await fetchSquadDashboardSlice(authToken)
      setData((prev) => (prev ? { ...prev, ...nextSlice } : prev))
      setError('')
    } catch (cause) {
      if (isUnauthorizedError(cause)) {
        markUnauthorized()
        return
      }

      setError(toDisplayError(cause, 'Не удалось обновить данные сквада.'))
    }
  }, [authToken, data, markUnauthorized, reload])

  const logout = useCallback(() => {
    clearAuthSession()
    navigateTo('/', { replace: true })
  }, [])

  useEffect(() => {
    const run = async () => {
      if (!authHydrated) return

      const tokenFromUrl = query.get('token')
      if (tokenFromUrl) {
        if (processedUrlTokenRef.current === tokenFromUrl) return
        processedUrlTokenRef.current = tokenFromUrl

        if (!validateTokenFormat(tokenFromUrl)) {
          setError('Некорректный токен авторизации.')
          setStatus('error')
          return
        }

        setAuthToken(tokenFromUrl)
        setStatus('loading')
        await loadDashboard(tokenFromUrl)
        const url = new URL(window.location.href)
        url.searchParams.delete('token')
        replaceUrl(url.pathname + url.search)
        return
      }

      processedUrlTokenRef.current = null
      if (!authToken) {
        redirectToAuth(getCurrentAppPath())
        return
      }

      setStatus('loading')
      await loadDashboard(authToken)
    }

    void run()
  }, [authHydrated, authToken, loadDashboard, query])

  return {
    authToken,
    status,
    data,
    error,
    setData,
    setAuthUser,
    reload,
    reloadSquads,
    logout,
  }
}
