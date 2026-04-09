import { useCallback, useEffect, useRef, useState } from 'react'
import { ApiError, toDisplayError } from '../../api/http'
import { currentAppPath, redirectToAuth } from '../../routes/auth'
import {
  getMySquad,
  getMySquadInvites,
  getSquadConfig,
  getSquadMembers,
} from '../../api/profile'
import { getCurrentUser } from '../../api/users'
import { tokenManager } from '../../services/tokenManager'
import { useAuthStore } from '../../store/authStore'
import type { ProfileDashboardData, ProfileStatus } from './types'

export function useProfileDashboard(query: URLSearchParams) {
  const authHydrated = useAuthStore((store) => store.hydrated)
  const authToken = useAuthStore((store) => store.token)
  const setAuthUser = useAuthStore((store) => store.setUser)
  const setAuthToken = useAuthStore((store) => store.setToken)
  const processedUrlTokenRef = useRef<string | null>(null)

  const [status, setStatus] = useState<ProfileStatus>('loading')
  const [data, setData] = useState<ProfileDashboardData | null>(null)
  const [error, setError] = useState('')

  const loadDashboard = useCallback(async (token: string) => {
    try {
      const [user, squad, squadInvites, squadConfig] = await Promise.all([
        getCurrentUser(token),
        getMySquad(token),
        getMySquadInvites(token),
        getSquadConfig(),
      ])

      const squadMembers = squad ? await getSquadMembers(token, squad.id) : []
      setAuthUser({ id: user.id, username: user.username, avatarUrl: user.avatarUrl ?? '' })
      setError('')
      setData({
        user,
        squad,
        squadMembers,
        squadInvites,
        squadConfig,
      })
      setStatus('loaded')
    } catch (cause) {
      if (cause instanceof ApiError && cause.isAuthError()) {
        void tokenManager.clearToken()
        setAuthToken(null)
        setAuthUser(null)
        setStatus('unauthorized')
        return
      }

      setError(toDisplayError(cause, 'Не удалось загрузить профиль.'))
      setStatus('error')
    }
  }, [setAuthToken, setAuthUser])

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
      const [squad, squadInvites, squadConfig] = await Promise.all([
        getMySquad(authToken),
        getMySquadInvites(authToken),
        getSquadConfig(),
      ])

      const squadMembers = squad ? await getSquadMembers(authToken, squad.id) : []

      setData((prev) => {
        if (!prev) {
          return prev
        }

        return {
          ...prev,
          squad,
          squadMembers,
          squadInvites,
          squadConfig,
        }
      })
      setError('')
    } catch (cause) {
      if (cause instanceof ApiError && cause.isAuthError()) {
        void tokenManager.clearToken()
        setAuthToken(null)
        setAuthUser(null)
        setStatus('unauthorized')
        return
      }

      setError(toDisplayError(cause, 'Не удалось обновить данные сквада.'))
    }
  }, [authToken, data, reload, setAuthToken, setAuthUser])

  const logout = useCallback(() => {
    void tokenManager.clearToken()
    setAuthToken(null)
    setAuthUser(null)
    window.location.assign('/')
  }, [setAuthToken, setAuthUser])

  useEffect(() => {
    const run = async () => {
      if (!authHydrated) return

      const tokenFromUrl = query.get('token')
      if (tokenFromUrl) {
        if (processedUrlTokenRef.current === tokenFromUrl) return
        processedUrlTokenRef.current = tokenFromUrl

        if (!tokenManager.validateTokenFormat(tokenFromUrl)) {
          setError('Некорректный токен авторизации.')
          setStatus('error')
          return
        }

        await tokenManager.storeToken(tokenFromUrl)
        setAuthToken(tokenFromUrl)
        setStatus('loading')
        await loadDashboard(tokenFromUrl)
        const url = new URL(window.location.href)
        url.searchParams.delete('token')
        window.history.replaceState(null, '', url.pathname + url.search)
        return
      }

      processedUrlTokenRef.current = null
      if (!authToken) {
        redirectToAuth(currentAppPath())
        return
      }

      setStatus('loading')
      await loadDashboard(authToken)
    }

    void run()
  }, [authHydrated, authToken, loadDashboard, query, setAuthToken])

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
